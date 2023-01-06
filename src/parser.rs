use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::Path;
use std::{fmt, fs};

use full_moon::ast::punctuated::{Pair, Punctuated};
use full_moon::ast::span::ContainedSpan;
use full_moon::ast::types::TypeDeclaration;
use full_moon::ast::{self, Expression, Field, Suffix, TableConstructor, Var};
use full_moon::node::Node;
use full_moon::tokenizer::{StringLiteralQuoteType, Symbol, Token, TokenReference, TokenType};
use full_moon::visitors::VisitorMut;

use crate::path::parse_path;

enum ModuleType {
    Directory,
    Lua,
    Json,
}

const HEADER: &str = include_str!("lua/header.lua");
#[derive(Debug, Clone)]
struct RequireError {
    value: String,
}

impl RequireError {
    pub fn new(value: String) -> Self {
        Self { value }
    }
}

impl fmt::Display for RequireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RequireError: {}", self.value)
    }
}

impl Error for RequireError {
    fn description(&self) -> &str {
        &self.value
    }
}

fn get_module_path(src_dir: &str, file_name: &str) -> Result<(String, ModuleType), String> {
    let file_path = format!("{}/{}.lua", src_dir, file_name);

    if Path::new(&file_path).exists() {
        return Ok((file_path, ModuleType::Lua));
    }

    let dir_path = format!("{}/{}/init.lua", src_dir, file_name);
    if Path::new(&dir_path).exists() {
        return Ok((dir_path, ModuleType::Directory));
    }

    let json_path = format!("{}/{}.json", src_dir, file_name);
    if Path::new(&json_path).exists() {
        return Ok((json_path, ModuleType::Json));
    }

    Err(format!("Module '{}' not found", file_name))
}

pub fn json_to_lua(json: &serde_json::Value) -> ast::Value {
    match json {
        serde_json::Value::Object(obj) => {
            let table = TableConstructor::new();
            let mut punctuated = Punctuated::<Field>::new();

            for (key, value) in obj {
                let expression = Expression::Value {
                    value: Box::new(json_to_lua(value)),
                    type_assertion: None,
                };

                let key = TokenReference::new(
                    vec![Token::new(TokenType::Symbol {
                        symbol: Symbol::LeftBracket,
                    })],
                    Token::new(TokenType::StringLiteral {
                        literal: key.into(),
                        multi_line: None,
                        quote_type: StringLiteralQuoteType::Double,
                    }),
                    vec![Token::new(TokenType::Symbol {
                        symbol: Symbol::RightBracket,
                    })],
                );

                punctuated.push(Pair::new(
                    Field::NameKey {
                        key,
                        equal: TokenReference::symbol("=").unwrap(),
                        value: expression,
                    },
                    Some(TokenReference::symbol(",").unwrap()),
                ));
            }

            ast::Value::TableConstructor(table.with_fields(punctuated))
        }
        serde_json::Value::Array(arr) => {
            let table = TableConstructor::new();
            let mut punctuated = Punctuated::<Field>::new();

            for entry in arr {
                punctuated.push(Pair::new(
                    Field::NoKey(Expression::Value {
                        value: Box::from(json_to_lua(entry)),
                        type_assertion: None,
                    }),
                    Some(TokenReference::symbol(",").unwrap()),
                ));
            }

            ast::Value::TableConstructor(table.with_fields(punctuated))
        }
        serde_json::Value::Bool(bool) => ast::Value::Symbol(TokenReference::new(
            Vec::new(),
            Token::new(TokenType::Symbol {
                symbol: if *bool { Symbol::True } else { Symbol::False },
            }),
            Vec::new(),
        )),
        serde_json::Value::Number(num) => ast::Value::Number(TokenReference::new(
            Vec::new(),
            Token::new(TokenType::Number {
                text: num.to_string().into(),
            }),
            Vec::new(),
        )),
        serde_json::Value::String(str) => ast::Value::String(TokenReference::new(
            Vec::new(),
            Token::new(TokenType::StringLiteral {
                literal: str.to_string().into(),
                multi_line: None,
                quote_type: StringLiteralQuoteType::Double,
            }),
            Vec::new(),
        )),
        serde_json::Value::Null => ast::Value::Symbol(TokenReference::new(
            Vec::new(),
            Token::new(TokenType::Symbol {
                symbol: Symbol::Nil,
            }),
            Vec::new(),
        )),
    }
}

pub struct RequireVisitor<'a> {
    // Parsing information
    src_dir: &'a str,
    entry_file: &'a str,

    // Keeping track of current state
    cur_file: String,         // as a relative path, from src_dir, without extension
    cur_imports: Vec<String>, // as a relative path, from cur_file, so like ./../hello/.., without extension
    cur_errors: Vec<String>,

    // Final state
    imports_memo: HashMap<String, Vec<String>>, // as a relative path, from the src_dir, without extension
    transformed_memo: HashMap<String, String>, // as a relative path, from the src_dir, without extension. This is the transformed lua code
    all_json: HashMap<String, String>, // same as all_imports, but for filename to lua table of json
}

impl<'a> RequireVisitor<'a> {
    pub fn new(src_dir: &'a str, entry_file: &'a str) -> Self {
        Self {
            src_dir,
            entry_file,

            cur_file: src_dir.to_string(),
            cur_imports: Vec::new(),
            cur_errors: Vec::new(),

            imports_memo: HashMap::new(),
            transformed_memo: HashMap::new(),
            all_json: HashMap::new(),
        }
    }

    /// Removes a file from the cached, and rebuilds the project
    pub fn mark_file_change(&mut self, file: &str) {
        self.imports_memo.remove(file);
        self.transformed_memo.remove(file);
        self.all_json.remove(file);
    }

    /// Builds the project
    pub fn generate_bundle(
        &mut self,
        development: bool,
    ) -> Result<(String, Vec<usize>, Vec<String>), Box<dyn Error>> {
        // Traverse the file tree to get the imports
        let mut imports = self.traverse()?;
        let mut bundle = String::from(HEADER);
        let mut source_map: Vec<usize> = Vec::new();

        source_map.push(HEADER.split("\n").count());

        // If we are in development, add the development code
        let mut dev_file_exists = false;
        if development {
            // Find if the development file exists
            let dev_file_path = format!("{}/{}", self.src_dir, ".dev.lua");
            if Path::new(&dev_file_path).exists() {
                dev_file_exists = true;
            }
        }

        if dev_file_exists {
            imports.push(String::from(".dev"));
        }

        // Add every import
        for import in &imports {
            let (module_path, module_type) = get_module_path(self.src_dir, &import)?;

            let module_content = match module_type {
                ModuleType::Lua | ModuleType::Directory => {
                    match self.transformed_memo.get(import) {
                        Some(lua) => lua.to_string(),
                        None => fs::read_to_string(module_path)?,
                    }
                }
                ModuleType::Json => {
                    let json_lua = self.all_json.get(import).unwrap();
                    json_lua.to_string()
                }
            };

            if let ModuleType::Directory = module_type {
                let dir_header = format!("\n__LUAJOIN_DIRECTORIES[\"{}\"]=true", import);
                bundle.push_str(&dir_header)
            }

            let import_header = format!("\n__LUAJOIN_FILES[\"{}\"]=function(_require)\n", import);
            let import_footer = "\nend";

            bundle.push_str(&(import_header + &module_content + import_footer));

            // Add the data to the source map
            source_map.push(bundle.split('\n').count());
        }

        // Add the dev footer
        if dev_file_exists {
            bundle.push_str("\n__LUAJOIN_FILES[\".dev\"](__LUAJOIN_require)");
        }

        // Add the footer, which will require the entry file
        bundle.push_str(&format!(
            "\n__LUAJOIN_FILES[\"{}\"](__LUAJOIN_require)\n",
            self.entry_file
        ));

        // prepend something
        imports.insert(0, String::from("[BUNDLER]"));

        Ok((bundle, source_map, imports))
    }

    /// Traverse the file tree, to return a list of all the files that are imported
    pub fn traverse(&mut self) -> Result<Vec<String>, Box<dyn Error>> {
        // First, clear the temporary storages
        {
            self.cur_imports.clear();
            self.cur_errors.clear();
        }

        let mut i = 0;
        let mut all_file_imports = vec![self.entry_file.to_string()];
        let mut all_file_imports_set: HashSet<String> = HashSet::new();

        while i < all_file_imports.len() {
            let import = all_file_imports.get(i).unwrap();

            // Get the import's file
            let (module_path, module_type) = get_module_path(self.src_dir, &import)?;

            // if it's json do something else
            if let ModuleType::Json = module_type {
                // If it's already parsed, then we don't need to visit it again
                if self.all_json.contains_key(import) {
                    i += 1;
                    continue;
                }

                let module_content = fs::read_to_string(&module_path)?;

                // Parse the json
                let json = serde_json::from_str(&module_content)?;
                let lua = json_to_lua(&json).to_string();

                self.all_json
                    .insert(import.clone(), "return ".to_owned() + &lua);

                i += 1;
                continue;
            }

            // If it's already visited, then we don't need to visit it again
            if self.imports_memo.contains_key(import) {
                let import_memo = self.imports_memo.get(import).unwrap();
                for import in import_memo {
                    // Only insert the one's that are not there yet
                    if all_file_imports_set.contains(import) {
                        continue;
                    }

                    all_file_imports_set.insert(import.clone());
                    all_file_imports.push(import.clone());
                }

                i += 1;
                continue;
            }

            // If it's not visited, then visit it
            let module_content = fs::read_to_string(&module_path)?;
            let module_ast = full_moon::parse(&module_content)?;

            self.cur_file = import.clone();
            self.cur_imports.clear();
            self.cur_errors.clear();

            let new_ast = self.visit_ast(module_ast);

            // If there's errors, then we can't continue
            if !self.cur_errors.is_empty() {
                let first_error = self.cur_errors.get(0).unwrap().clone().to_string();

                return Err(Box::new(RequireError::new(format!(
                    "'{}': {}",
                    import, first_error
                ))));
            }

            // Parse all the relative imports
            let mut rel_imports: Vec<String> = Vec::new();
            let mut rel_imports_set: HashSet<String> = HashSet::new();

            for import in &self.cur_imports {
                let path = match module_type {
                    ModuleType::Directory => {
                        parse_path(&format!("{}/init", &self.cur_file), import)
                    }
                    ModuleType::Lua => parse_path(&self.cur_file, import),
                    _ => panic!("Unknown module type"),
                };

                if !rel_imports_set.contains(&path) {
                    rel_imports_set.insert(path.clone());
                    rel_imports.push(path.clone());
                }

                if !all_file_imports_set.contains(&path) {
                    all_file_imports_set.insert(path.clone());
                    all_file_imports.push(path.clone());
                }
            }

            // Transform the AST
            let new_source = full_moon::print(&new_ast);

            self.transformed_memo
                .insert(self.cur_file.clone(), new_source);
            self.imports_memo.insert(self.cur_file.clone(), rel_imports);

            i += 1;
        }

        Ok(all_file_imports)
    }
}

fn empty_token(lines: usize) -> Token {
    Token::new(TokenType::Whitespace {
        characters: "\n".repeat(lines).into(),
    })
}

fn empty_token_ref(lines: usize) -> TokenReference {
    TokenReference::new(Vec::new(), empty_token(lines), Vec::new())
}

impl<'a> VisitorMut for RequireVisitor<'a> {
    // Remove every comment from the AST
    fn visit_multi_line_comment(&mut self, token: Token) -> Token {
        empty_token(token.to_string().split("\n").count() - 1)
    }

    // Single line comments too
    fn visit_single_line_comment(&mut self, _: Token) -> Token {
        empty_token(0)
    }

    // Remove the type specifiers (function foo(bar: string)), remove the : string
    fn visit_type_specifier(&mut self, _: ast::types::TypeSpecifier) -> ast::types::TypeSpecifier {
        ast::types::TypeSpecifier::new(ast::types::TypeInfo::Basic(empty_token_ref(0)))
            .with_punctuation(empty_token_ref(0))
    }

    // type assertions: a::string
    fn visit_type_assertion(&mut self, _: ast::types::TypeAssertion) -> ast::types::TypeAssertion {
        ast::types::TypeAssertion::new(ast::types::TypeInfo::Basic(empty_token_ref(1)))
            .with_assertion_op(empty_token_ref(0))
    }

    // declarations (type a = string)
    fn visit_type_declaration(
        &mut self,
        token: ast::types::TypeDeclaration,
    ) -> ast::types::TypeDeclaration {
        TypeDeclaration::new(
            empty_token_ref(0),
            ast::types::TypeInfo::Basic(empty_token_ref(0)),
        )
        .with_equal_token(empty_token_ref(0))
        .with_type_token(empty_token_ref(token.to_string().split('\n').count() - 1))
    }

    fn visit_function_call(&mut self, node: ast::FunctionCall) -> ast::FunctionCall {
        // Make sure it's a '_require' call
        if let ast::Prefix::Name(name) = node.prefix() {
            if let TokenType::Identifier { identifier } = name.token_type() {
                if identifier.to_string() != "_require" && identifier.to_string() != "require" {
                    return node;
                }
            }
        }

        // Get the arguments
        if let ast::Suffix::Call(ast::Call::AnonymousCall(ast::FunctionArgs::Parentheses {
            parentheses,
            arguments,
        })) = node.suffixes().next().unwrap()
        {
            let first_arg = match arguments.iter().next() {
                Some(arg) => arg,
                None => {
                    self.cur_errors
                        .push(String::from("An argument is required for '_require'"));

                    return node.clone();
                }
            };

            // Extract the first arg from the quotes
            if let ast::Expression::Value {
                value,
                type_assertion: _,
            } = first_arg
            {
                match *value.clone() {
                    ast::Value::String(s) => {
                        if let TokenType::StringLiteral {
                            literal,
                            multi_line: _,
                            quote_type: _,
                        } = s.token_type()
                        {
                            // Get the string literal
                            let required_path = literal.to_string();

                            // Add it to the imports
                            self.cur_imports.push(required_path.clone());
                        }
                    }
                    ast::Value::Var(Var::Expression(ve)) => {
                        let parts = ve.tokens().into_iter();

                        // Will store the state for the relative import, later joined into a string
                        let mut rel_import_path: Vec<String> = Vec::new();

                        for part in parts {
                            if let TokenType::Identifier { identifier } = part.token_type() {
                                let part = identifier.to_string();
                                if part == "script" {
                                    rel_import_path.push(String::from("."));
                                } else if part == "Parent" {
                                    rel_import_path.push(String::from(".."));
                                } else {
                                    rel_import_path.push(part);
                                }
                            }
                        }

                        // Make sure that the current require has the first part as 'script'
                        if rel_import_path[0] != "." {
                            return node.clone();
                        }

                        let required_path = rel_import_path.join("/");
                        self.cur_imports.push(required_path.clone());

                        let mut punctuated = Punctuated::new();
                        punctuated.push(Pair::new(
                            ast::Expression::Value {
                                value: Box::new(ast::Value::String(TokenReference::new(
                                    Vec::new(),
                                    Token::new(TokenType::StringLiteral {
                                        literal: required_path.into(),
                                        multi_line: None,
                                        quote_type: StringLiteralQuoteType::Double,
                                    }),
                                    Vec::new(),
                                ))),
                                type_assertion: None,
                            },
                            None,
                        ));

                        let ends_with_newline =
                            match parentheses.tokens().1.trailing_trivia().last() {
                                Some(t) => {
                                    if let TokenType::Whitespace { characters } = t.token_type() {
                                        characters.to_string().contains("\n")
                                    } else {
                                        false
                                    }
                                }
                                _ => false,
                            };

                        return node
                            .clone()
                            .with_prefix(ast::Prefix::Name(TokenReference::new(
                                Vec::new(),
                                Token::new(TokenType::Identifier {
                                    identifier: "_require".into(),
                                }),
                                Vec::new(),
                            )))
                            .with_suffixes(vec![Suffix::Call(ast::Call::AnonymousCall(
                                ast::FunctionArgs::Parentheses {
                                    parentheses: ContainedSpan::new(
                                        TokenReference::new(
                                            Vec::new(),
                                            Token::new(TokenType::Identifier {
                                                identifier: "(".into(),
                                            }),
                                            Vec::new(),
                                        ),
                                        TokenReference::new(
                                            Vec::new(),
                                            Token::new(TokenType::Identifier {
                                                identifier: ")".into(),
                                            }),
                                            if ends_with_newline {
                                                vec![
                                                    (Token::new(TokenType::Whitespace {
                                                        characters: "\n".into(),
                                                    })),
                                                ]
                                            } else {
                                                Vec::new()
                                            },
                                        ),
                                    ),
                                    arguments: punctuated,
                                },
                            ))]);

                        // TODO: the token into a _require
                    }
                    _ => (),
                };
            }
        }

        return node;
    }
}
