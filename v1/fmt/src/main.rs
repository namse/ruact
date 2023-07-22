use anyhow::Result;
use nom::{
    bytes::complete::tag,
    character::complete::multispace0,
    error::{ParseError, VerboseError},
    multi::many0,
    sequence::delimited,
    IResult,
};
use std::process::Stdio;
use tokio::{
    io::{self, AsyncReadExt},
    process::Command,
};

#[tokio::main]
async fn main() -> Result<()> {
    let rustfmt = Command::new("rustfmt")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut stdin = rustfmt.stdin.unwrap();
    let mut stdout = rustfmt.stdout.unwrap();

    tokio::try_join![
        tokio::spawn(async move {
            io::copy(&mut io::stdin(), &mut stdin)
                .await
                .map_err(|e| anyhow::Error::from(e))
                .unwrap();
        }),
        tokio::spawn(async move {
            let mut rustfmted = String::new();
            stdout
                .read_to_string(&mut rustfmted)
                .await
                .map_err(|e| anyhow::Error::from(e))
                .unwrap();

            let formatted = format_rsx(rustfmted);
            println!("{formatted}");
        })
    ]?;

    Ok(())
}

fn ws<'a, F, O, E: ParseError<&'a str>>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}

fn parse_value_in_string<'a, Error: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, &'a str, Error> {
    let (input, value) = ws(nom::character::complete::alphanumeric1)(input)?;
    Ok((input, value))
}

#[derive(Debug)]
struct Field<'a> {
    name: &'a str,
    value: &'a str,
}

fn parse_field<'a, Error: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Field, Error> {
    let (input, name) = ws(nom::character::complete::alpha1)(input)?;
    let (input, _) = ws(tag(":"))(input)?;
    let (input, value) = ws(parse_value_in_string)(input)?;

    Ok((input, Field { name, value }))
}

#[derive(Debug)]
struct Component<'a> {
    name: &'a str,
    fields: Vec<Field<'a>>,
    children: Vec<Component<'a>>,
}
fn parse_component<'a, Error: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, Component, Error> {
    let (input, component_name) = ws(nom::character::complete::alpha1)(input)?;

    let (input, fields) = nom::combinator::opt(ws(nom::sequence::delimited(
        ws(tag("{")),
        ws(nom::multi::many0(parse_field)),
        ws(tag("}")),
    )))(input)?;

    let (input, children) = nom::combinator::opt(ws(nom::sequence::delimited(
        ws(tag("(")),
        ws(nom::multi::many0(parse_component)),
        ws(tag(")")),
    )))(input)?;

    let (input, _) = nom::combinator::opt(ws(tag(",")))(input)?;

    let component = Component {
        name: component_name,
        fields: fields.unwrap_or_default(),
        children: children.unwrap_or_default(),
    };

    Ok((input, component))
}

#[derive(Debug)]
struct Root<'a> {
    components: Vec<Component<'a>>,
}

fn parse_root<'a, Error: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Root, Error> {
    let (input, _) = tag("rsx!")(input)?;

    let (input, components) = nom::sequence::delimited(
        ws(nom::branch::alt((tag("["), tag("("), tag("{")))),
        many0(ws(parse_component)),
        ws(nom::branch::alt((tag("]"), tag(")"), tag("}")))),
    )(input)?;

    Ok((input, Root { components }))
}

fn format_rsx(file: String) -> String {
    let mut input: &str = &file;

    while let Some(index) = input.find("rsx!") {
        let indent = input[..index].rfind('\n').map_or(0, |i| index - i - 1);
        print!("{}", &input[..index]);
        input = &input[index..];
        match parse_root::<VerboseError<_>>(input) {
            Ok((rest, root)) => {
                print_root(root, indent);
                input = rest;
            }
            Err(error) => {
                eprintln!("error: {:#?}", error);
                print!("{}", &input[index..]);
                break;
            }
        }
    }
    print!("{input}");

    file
}

// print like this.
// rsx!(
//     Button {
//         value: 5
//     }(
//         Button { value: 5 },
//         Button { value: 5 },
//     ),
// )

fn print_root(root: Root, indent: usize) {
    // TODO: Use rustfmt to format components
    println!("rsx!(");

    root.components.into_iter().for_each(|component| {
        print_component(component, indent + 4);
    });

    println!("{:indent$})", "");
}

fn print_component(component: Component, indent: usize) {
    println!("{:indent$}{name} {{", "", name = component.name,);

    component.fields.into_iter().for_each(|field| {
        println!(
            "{:indent$}{name}: {value},",
            "",
            name = field.name,
            value = field.value,
            indent = indent + 4
        );
    });

    print!("{:indent$}}}", "");

    if !component.children.is_empty() {
        println!("(");

        component.children.into_iter().for_each(|child| {
            print_component(child, indent + 4);
        });

        print!("{:indent$})", "");
    }

    println!(",");
}
