use std::{
    error::Error,
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::PathBuf,
};

const MAGIC_COMMENT_PREFIX: &str = "//backpack:";

const FORWARD_TO_MAGIC_COMMENT: &str = "forward-to ";

pub fn on_build() -> Result<(), Box<dyn Error>> {
    let project_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    let build_dir = std::env::var("OUT_DIR")?;

    let build_dir = PathBuf::from(build_dir);

    let project_dir = PathBuf::from(project_dir);
    if !project_dir.exists() {
        Err(format!(
            "project dir does not exist at: '{}'",
            project_dir.display()
        ))?
    }

    let mut proxy_functions_src_path = project_dir.clone();
    proxy_functions_src_path.push("src");
    proxy_functions_src_path.push("proxyfunctions.rs");

    let config = ProxyConfig::parse(&proxy_functions_src_path)
        .map_err(|err| format!("failed to parse proxy functions source file - {err}"))?;

    let mut windows_def_path = build_dir.clone();
    windows_def_path.push("def.txt");

    let mut windows_def_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&windows_def_path)
        .map_err(|err| format!("failed to open def file for writing - {err}"))?;

    write!(windows_def_file, "{}", config.to_windows_def_file())
        .map_err(|err| format!("failed to write def file contents - {err}"))?;

    let mut build_opts = String::new();

    build_opts.push_str("cargo::rustc-link-arg-cdylib=/DEF:");
    build_opts.push_str(windows_def_path.display().to_string().as_str());

    println!("{build_opts}");

    Ok(())
}

struct ProxyConfig {
    forward_to_library: String,
    functions: Vec<Function>,
}

struct Function {
    name: String,
}

impl ProxyConfig {
    fn parse(functions_src_path: &PathBuf) -> Result<Self, Box<dyn Error>> {
        ProxyConfigParser::parse_path(functions_src_path)
    }

    fn to_windows_def_file(&self) -> String {
        let mut def = String::new();

        const NEWLINE: &str = "\r\n";

        // EXPORTS
        //    add=targetlib_orig.add
        def.push_str("EXPORTS");
        def.push_str(NEWLINE);

        for func in self.functions.iter().enumerate() {
            def.push_str("    ");

            def.push_str(&func.1.name);

            def.push('=');

            def.push_str(&self.forward_to_library);

            def.push('.');

            def.push_str(&func.1.name);

            def.push_str(NEWLINE);
        }

        def
    }
}

struct ProxyConfigParser {
    forward_to_library: Option<String>,
    current_func: Option<FunctionAttributes>,
    functions: Vec<Function>,
}

impl ProxyConfigParser {
    fn parse_path(functions_src_path: &PathBuf) -> Result<ProxyConfig, Box<dyn Error>> {
        let src_file = File::open(functions_src_path)
            .map_err(|err| format!("failed to open proxy functions source file - {err}"))?;

        let mut buf_reader = BufReader::new(src_file);

        let mut parser = Self {
            forward_to_library: None,
            current_func: None,
            functions: Vec::new(),
        };

        parser.parse(&mut buf_reader)?;

        if parser.forward_to_library.is_none() {
            Err(format!(
                "missing magic comment: {}{}",
                MAGIC_COMMENT_PREFIX, FORWARD_TO_MAGIC_COMMENT
            ))?;
        }

        Ok(ProxyConfig {
            forward_to_library: parser.forward_to_library.unwrap(),
            functions: parser.functions,
        })
    }

    fn parse<R: io::BufRead>(&mut self, buf_reader: &mut R) -> Result<(), Box<dyn Error>> {
        for (line_num, line) in buf_reader.lines().enumerate() {
            let line = line.map_err(|err| format!("failed to read line - {err}"))?;

            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            self.parse_line(line)
                .map_err(|err| format!("line {}: {err}", line_num + 1))?;
        }

        Ok(())
    }

    fn parse_line(&mut self, line: &str) -> Result<(), Box<dyn Error>> {
        if line.starts_with("#[") {
            maybe_update_function_attributes(line, &mut self.current_func)?;
        } else if line.starts_with("fn ") {
            if let Some(current_func) = &self.current_func {
                let current_func = current_func.clone();

                self.current_func = None;

                if let Some(name) = current_func.name {
                    self.functions.push(Function { name: name });

                    return Ok(());
                }
            }

            let without_prefix = line.trim_start_matches("fn ");

            let split_by_parens: Vec<&str> = without_prefix.split('(').collect();

            if !split_by_parens.is_empty() {
                self.functions.push(Function {
                    name: split_by_parens[0].trim().to_string(),
                });
            }
        } else if line.starts_with(MAGIC_COMMENT_PREFIX) {
            let without_prefix = line.trim_start_matches(MAGIC_COMMENT_PREFIX);

            if without_prefix.starts_with(FORWARD_TO_MAGIC_COMMENT) {
                if self.forward_to_library.is_some() {
                    Err(format!("{FORWARD_TO_MAGIC_COMMENT} redeclared"))?;
                }

                let without_prefix = without_prefix
                    .trim_start_matches(FORWARD_TO_MAGIC_COMMENT)
                    .trim();

                self.forward_to_library = Some(without_prefix.to_string());
            } else {
                Err(format!(
                    "unknown / unsupported magic variable: {without_prefix}"
                ))?;
            }
        }

        Ok(())
    }
}

fn maybe_update_function_attributes(
    line: &str,
    function: &mut Option<FunctionAttributes>,
) -> Result<(), Box<dyn Error>> {
    // #[unsafe(export_name = "typeof")]
    // #[unsafe(no_mangle)]
    let line = line.trim_start_matches("#[");
    let line = line.trim_end_matches("]");

    let start_len = line.len();

    let mut inner_str = line.trim_start_matches("unsafe(");

    if start_len != inner_str.len() {
        inner_str = inner_str.trim_end_matches(")");
    }

    for kv_str in inner_str.split(",") {
        match kv_str.split_once("=") {
            Some(kv) => {
                let key = kv.0.trim();

                let val = kv.1.trim().trim_matches('"');

                match key {
                    "export_name" => {
                        let attribs = function.get_or_insert(FunctionAttributes { name: None });
                        attribs.name = Some(val.to_string());
                    }
                    _ => {}
                }
            }
            None => {}
        }
    }

    Ok(())
}

#[derive(Clone)]
struct FunctionAttributes {
    name: Option<String>,
}
