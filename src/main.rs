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
        Rule::code_block => {
            buf += "<pre>";
            let tk_lst: Vec<String> = token.into_inner().map(parse_content).collect();
            buf += &tk_lst.join("");
            buf += "</pre>";
        }
        Rule::raw_block => {
            let raw = token.as_str();
            buf += &raw[4..raw.len().saturating_sub(4)];
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

// Read a file fully with a buffered reader.
fn buf_read(path: &Path) -> Result<String, std::io::Error> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    let length = metadata.len();
    let mut buf: Vec<u8> = Vec::with_capacity(length as usize);
    let mut rdr = BufReader::new(file);
    rdr.read_to_end(&mut buf)?;
    String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
}

// Write a string in a file, creating it if it does not exist
// and replacing it if it does. Use a bufwriter for perf.
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

// Copy all the file from a directory to another.
// Both directory should already exist.
fn copy_files_dir(source_dir: &Path, target_dir: &Path) -> io::Result<()> {
    for entry in fs::read_dir(source_dir)? {
        let path = entry?.path();
        if path.is_file() {
            let mut dest_path = PathBuf::from(target_dir);
            dest_path.push(path.file_name().unwrap());
            fs::copy(&path, &dest_path)?;
        }
    }
    Ok(())
}

// Return a tuple with the basename and the extension
fn parse_filename(filename: &String) -> (&str, &str) {
    let fext_re = Regex::new(r"(.*)\.(.+)$").unwrap();
    let captures = fext_re.captures(filename).unwrap();
    let basename = captures.get(1).unwrap().as_str();
    let ext = captures.get(2).unwrap().as_str();
    (basename, ext)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if let Some(a) = args.get(1) {
        if a == "help" {
            println!("USAGES: <templates directory> <pages directory> <static file directory>");
            return;
        }
    }

    let templates_dir = args
        .get(1)
        .map(|e| e.clone())
        .unwrap_or("templates".to_string());
    let pages_dir = args
        .get(2)
        .map(|e| e.clone())
        .unwrap_or("pages".to_string());
    let static_dir = PathBuf::from(
        args.get(3)
            .map(|e| e.clone())
            .unwrap_or("static".to_string()),
    );

    let mut jinja = Environment::new();

    // Crawl and register all the templates
    let mut templates: HashMap<String, String> = HashMap::new();
    for entry in WalkDir::new(&templates_dir).follow_links(true) {
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
    for entry in WalkDir::new(&pages_dir).follow_links(true) {
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
    let mut pages_meta: Vec<HashMap<String, String>> = Vec::new();
    for (fname, page) in pages.iter_mut() {
        println!("page {}, metadata: {:?}", fname, page.metadata);
        let errmsg = format!(
            "Page ({}) metadata does not contain a template file.",
            fname
        );
        let template_name = page.metadata.get("template").expect(&errmsg);

        // Get the template extension.
        let (_, template_ext) = parse_filename(template_name);

        // Get the page base filename (no ext)
        let (page_base_filename, _) = parse_filename(fname);

        let mut ctx = page.metadata.clone();
        ctx.insert("content".to_string(), page.content.clone());

        let final_path = format!("{}.{}", page_base_filename, template_ext).to_string();
        page.metadata.insert("path".to_string(), final_path);
        pages_meta.push(page.metadata.clone());
    }

    // Render all the pages
    // TODO: handle duplicates file name.
    let mut render_dir_path = PathBuf::new();
    render_dir_path.push(".");
    render_dir_path.push("render");

    match fs::create_dir(&render_dir_path) {
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

        let mut path = render_dir_path.clone();
        path.push(ctx.get("path").unwrap());
        let rendered = tmpl
            .render(context! { page => ctx, global => pages_meta })
            .expect("Failed to render.");
        write_string_to_file(&path, &rendered).expect("Failed to write rendered template.");
    }

    // Copy all the static files
    copy_files_dir(&static_dir, &render_dir_path).expect("Failed to copy static files.");
}
