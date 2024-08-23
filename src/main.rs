use std::collections::HashMap;
use std::env;
use std::fs;

use minijinja::{context, Environment};

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use walkdir::WalkDir;

#[derive(Parser)]
#[grammar = "maeldown.pest"]
pub struct MDParser;


pub struct Page {
    pub metadata: HashMap<String, String>,
    pub content: String,
}

impl Page {
    pub fn empty() -> Self {
        Page {
            metadata: HashMap::new(),
            content: String::new(),
        }
    }
}

fn parse_content(token: Pair<Rule>) -> String {
    let mut buf = String::new();

    match token.as_rule() {
        Rule::heading => {
            buf += "<h1>";
            let tk_lst: Vec<String> = token.into_inner().map(parse_content).collect();
            buf += &tk_lst.join("");
            buf += "</h1>";
        }
        Rule::char => {
            buf += token.as_str();
        }
        Rule::text_block => {
            buf += "<p>";
            let tk_lst: Vec<String> = token.into_inner().map(parse_content).collect();
            buf += &tk_lst.join("");
            buf += "</p>";
        }
        Rule::bold_text => {
            buf += "<span class=\"bold\">";
            let tk_lst: Vec<String> = token.into_inner().map(parse_content).collect();
            buf += &tk_lst.join("");
            buf += "</span>";
        }
        Rule::italic_text => {
            buf += "<span class=\"italic\">";
            let tk_lst: Vec<String> = token.into_inner().map(parse_content).collect();
            buf += &tk_lst.join("");
            buf += "</span>";
        }
        Rule::sidenote => {
            buf += "<aside class=\"sidenote\">";
            let tk_lst: Vec<String> = token.into_inner().map(parse_content).collect();
            buf += &tk_lst.join("");
            buf += "</aside>";
        }
        Rule::link => {
            let mut inner = token.into_inner();
            let name = inner.next().unwrap();
            let url = inner.next().unwrap();
            buf += "<a href=\"";
            buf += &parse_content(url);
            buf += "\">";
            buf += &parse_content(name);
            buf += "</a>";
        }
        Rule::link_name => {
            buf += token.as_str();
        }
        Rule::link_url => {
            buf += token.as_str();
        }
        Rule::EOI => {}
        _ => {
            // println!("Unr: {:?}", token);
        }
    }

    buf
}

fn parse_metadata(token: Pair<Rule>, meta: &mut HashMap<String, String>) {
    for property in token.into_inner() {
        let mut inner = property.into_inner();
        let key = inner.next().unwrap().as_str().to_string();
        let value = inner.next().unwrap().as_str().to_string();
        meta.insert(key, value);
    }  
} 

fn parse_file(path: &str) -> Page {
    let unparsed_file = fs::read_to_string(path).expect("Could not read file");
    let parsed = MDParser::parse(Rule::document, &unparsed_file)
        .expect("could not parse")
        .next()
        .unwrap();
    
    let mut page = Page::empty();
    let root = parsed.into_inner();
    for token in root {
        match token.as_rule() {
            Rule::EOI => {},
            Rule::article => {
                for art in token.into_inner() {
                    page.content += &parse_content(art);
                }
            },
            Rule::metadata => {
                parse_metadata(token, &mut page.metadata);
            }
            _ => {},
        }
    }
    page
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("not enough argument: <template file> <article directory>");
        return;
    }

    // Load template
    let template_str = fs::read_to_string(&args[1]).expect("Could not read template file.");
    let mut env = Environment::new();
    env.add_template("base", &template_str).unwrap();
    let template = env.get_template("base").unwrap();

    // Parse all the articles
    let mut articles: Vec<String> = Vec::new();
    for entry in WalkDir::new(&args[2]).follow_links(true) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
            let article = fs::read_to_string(path);
            if let Err(e) = article {
                println!("Error while reading {}: {}", path.display(), e);
                continue;
            }
            let parsed = parse_file(path.to_str().unwrap());
            articles.push(parsed.content);
            println!("META: {:?}", parsed.metadata);
        }
    }

    // Render everything
    // TODO: Add metadata to maeldown and sort article by date.
    println!("{}", template.render(context! { title => "Wololo", articles => articles }).unwrap());
}
