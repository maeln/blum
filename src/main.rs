use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::{Read, Write};
use std::path::Path;
use std::path::PathBuf;

use minijinja::context;
use minijinja::Environment;

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use walkdir::WalkDir;

use regex::Regex;

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
            Rule::EOI => {}
            Rule::article => {
                for art in token.into_inner() {
                    page.content += &parse_content(art);
                }
            }
            Rule::metadata => {
                parse_metadata(token, &mut page.metadata);
            }
            _ => {}
        }
    }
    page
}

fn buf_read(path: &Path) -> Result<String, std::io::Error> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    let length = metadata.len();
    let mut buf: Vec<u8> = Vec::with_capacity(length as usize);
    let mut rdr = BufReader::new(file);
    rdr.read_to_end(&mut buf)?;
    String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
}

fn write_string_to_file(path: &Path, data: &str) -> io::Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(data.as_bytes())?;
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("not enough argument: <templates directory> <pages directory>");
        return;
    }

    let mut jinja = Environment::new();

    // Crawl and register all the templates
    let mut templates: HashMap<String, String> = HashMap::new();
    for entry in WalkDir::new(&args[1]).follow_links(true) {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let filename = path.file_name().unwrap().to_str().unwrap().to_string();
                templates.insert(
                    filename.clone(),
                    buf_read(path).expect("Could not read template file."),
                );
            }
            Err(e) => println!("Error while crawling tempalate directory: {}", e),
        }
    }

    for (fname, content) in templates.iter() {
        jinja
            .add_template(fname, content)
            .expect("Could not register template!");
    }

    // Parse all the pages
    let mut pages: HashMap<String, Page> = HashMap::new();
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
            let filename = path.file_name().unwrap().to_str().unwrap().to_string();
            pages.insert(filename, parsed);
        }
    }

    // Computing all the rendered file path and add every page metadata
    // and their path available to all template.
    let fext_re = Regex::new(r".*\.(.+)$").unwrap();
    let mut pages_meta: Vec<HashMap<String,String>> = Vec::new();
    for (fname, page) in pages.iter_mut() {
        let template_name = page
            .metadata
            .get("template")
            .expect("Page metadata does not contain a template file.");

        let template_ext_cap = fext_re
            .captures(&template_name)
            .expect("Failed to parse template name");
        let template_ext = template_ext_cap
            .get(1)
            .expect("Could not find template extendion")
            .as_str();

        let mut ctx = page.metadata.clone();
        ctx.insert("content".to_string(), page.content.clone());

        let mut path = PathBuf::new();
        path.push(".");
        path.push("render");
        path.push(format!("{}.{}", fname, template_ext).to_string());
        let str_path = path.to_str().unwrap().to_string();

        page.metadata.insert("path".to_string(), str_path);
        pages_meta.push(page.metadata.clone());
    }

    // Render all the pages
    // TODO: handle duplicates file name.

    match fs::create_dir("./render") {
        Ok(()) => {}
        Err(e) => {
            if e.kind() != io::ErrorKind::AlreadyExists {
                println!("Error while creating directory {}", e)
            }
        }
    };

    for (_, page) in pages.iter() {
        let template_name = page
            .metadata
            .get("template")
            .expect("Page metadata does not contain a template file.");
        let tmpl = jinja
            .get_template(template_name)
            .expect("Could not find the right template");

        let mut ctx = page.metadata.clone();
        ctx.insert("content".to_string(), page.content.clone());

        let path = PathBuf::from(ctx.get("path").unwrap());
        let rendered = tmpl.render(context! { page => ctx, global => pages_meta }).expect("Failed to render.");
        write_string_to_file(&path, &rendered).expect("Failed to write rendered template.");
    }
}
