use std::fs;

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "maeldown.pest"]
pub struct MDParser;

fn generate(token: Pair<Rule>) -> String {
    let mut buf = String::new();

    match token.as_rule() {
        Rule::heading => {
            buf += "<h1>";
            let tk_lst: Vec<String> = token.into_inner().map(generate).collect();
            buf += &tk_lst.join("");
            buf += "</h1>";
        }
        Rule::char => {
            buf += token.as_str();
        }
        Rule::text_block => {
            let tk_lst: Vec<String> = token.into_inner().map(generate).collect();
            buf += &tk_lst.join("");
        }
        Rule::bold_text => {
            buf += "<b>";
            let tk_lst: Vec<String> = token.into_inner().map(generate).collect();
            buf += &tk_lst.join("");
            buf += "</b>";
        }
        Rule::italic_text => {
            buf += "<i>";
            let tk_lst: Vec<String> = token.into_inner().map(generate).collect();
            buf += &tk_lst.join("");
            buf += "</i>";
        }
        Rule::sidenote => {
            println!("DEG: {:?}", token);
            buf += "<sidenote>";
            let tk_lst: Vec<String> = token.into_inner().map(generate).collect();
            buf += &tk_lst.join("");
            buf += "</sidenote>";
        }
        Rule::link => {
            let mut inner = token.into_inner();
            let name = inner.next().unwrap();
            let url = inner.next().unwrap();
            buf += "<a href=\"";
            buf += &generate(url);
            buf += "\">";
            buf += &generate(name);
            buf += "</a>";
        }
        Rule::link_name => {
            buf += token.as_str();
        }
        Rule::link_url => {
            buf += token.as_str();
        }
        Rule::EOI => {
            println!("EOI");
        }
        _ => {
            println!("Unr: {:?}", token);
        }
    }

    buf
}

fn main() {
    let unparsed_file = fs::read_to_string("test.md").expect("Could not read file");
    let parsed = MDParser::parse(Rule::document, &unparsed_file)
        .expect("could not parse")
        .next()
        .unwrap();

    let mut document = String::new();
    for line in parsed.into_inner() {
        document += &generate(line);
    }
    println!("{}", document);
}
