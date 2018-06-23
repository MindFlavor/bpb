extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use std::fs::File;
use std::io::prelude::*;

#[derive(Debug, Clone, Deserialize)]
pub struct Field {
    pub name: String,
    pub field_type: String,
    pub builder_type: Option<String>,
    pub optional: bool,
    pub initializer: Option<String>,
    pub trait_get: Option<String>,
    pub trait_set: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Struct {
    pub name: String,
    pub extra_types: Vec<String>,
    pub fields: Vec<Field>,
}

fn main() {
    let file_name = std::env::args()
        .nth(1)
        .expect("pass the json name as parameter");

    let stc: Struct = {
        let mut f = File::open(file_name).expect("file not found!");
        let mut contents = String::new();
        f.read_to_string(&mut contents).unwrap();
        serde_json::from_str(&contents).unwrap()
    };

    let mut output = String::new();

    // create the struct
    output.push_str(&format!(
        "pub struct {}{} {{\n",
        stc.name,
        calculate_type_description(&stc)
    ));

    for f in &stc.fields {
       output.push_str(&format!("\t{}: {},\n", f.name, calculate_type(f)));    
    }

    output.push_str("}\n\n");

    println!("{:?}", stc);
    println!("\n{}", output);
}

fn calculate_type(f: &Field) -> String {
    if !f.optional {
        return f.field_type.to_owned();
    }

    // not optional
    match f.initializer {
        Some(ref c) => f.field_type.to_owned(),
        None => format!("Option<{}>", f.field_type)
    }
}

fn calculate_type_description(stc: &Struct) -> String {
    let mut s = String::new();

    let mut fFirst = true;
    for f in &stc.extra_types {
        if !fFirst {
            s.push_str(", ");
        }

        s.push_str(&f);
        fFirst = false;
    }

    for f in stc.fields.iter().filter(|f| f.optional == false) {
        if !fFirst {
            s.push_str(", ");
        }

        let bt = match f.builder_type {
            Some(ref a) => a,
            None => panic!(),
        };

        s.push_str(&bt);
        fFirst = false;
    }

    if s.is_empty() {
        "".to_owned()
    } else {
        format!("<{}>", s)
    }
}
