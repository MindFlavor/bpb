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
pub struct ConstructorField {
    pub name: String,
    pub field_type: String,
    pub trait_get: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Struct {
    pub name: String,
    pub extra_types: Vec<String>,
    pub constructor_fields: Vec<ConstructorField>,
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

    let all_builder_types = {
        let mut abt = Vec::new();
        stc.fields
            .iter()
            .filter(|f| !f.optional)
            .map(|bt| abt.push(bt.builder_type.clone().unwrap()))
            .collect::<()>();
        abt
    };

    let mut output = String::new();

    // create the struct
    {
        output.push_str(&format!(
            "pub struct {}{}\n{} {{\n",
            stc.name,
            calculate_type_description(&stc, &[]),
            calculate_where(&stc)
        ));

        // constructor types
        for t in stc.constructor_fields.iter() {
            output.push_str(&format!("\t{}: {},\n", t.name, t.field_type));
        }

        // phantom types
        for f in stc.fields.iter().filter(|f| !f.optional) {
            output.push_str(&format!(
                "\tp_{}: PhantomData<{}>,\n",
                f.name,
                f.clone().builder_type.unwrap()
            ));
        }

        for f in &stc.fields {
            output.push_str(&format!("\t{}: {},\n", f.name, calculate_type(f)));
        }

        output.push_str("}\n\n");
    }

    // create the ctor
    {
        output.push_str(&format!(
            "impl{} for {}{} {{\n",
            calculate_type_description(&stc, &all_builder_types),
            stc.name,
            calculate_type_description_all_no(&stc)
        ));

        output.push_str(&format!(
            "\t pub(crate) fn new({}) -> {}{} {{\n\t\t{} {{\n",
            calculate_constructor_parameters(&stc),
            stc.name,
            calculate_type_description_all_no(&stc),
            stc.name
        ));

        for cp in stc.constructor_fields.iter() {
            output.push_str(&format!("\t\t\t{},\n", cp.name));
        }

        for f in stc.fields.iter().filter(|f| !f.optional) {
            output.push_str(&format!("\t\t\tp_{}: PhantomData {{}},\n", f.name));
            match f.initializer {
                Some(ref initializer) => {
                    output.push_str(&format!("\t\t\t{}: {},\n", f.name, initializer))
                }
                None => output.push_str(&format!("\t\t\t{}: None,\n", f.name)),
            };
        }

        for f in stc.fields.iter().filter(|f| f.optional) {
            output.push_str(&format!("\t\t\t{}: None\n", f.name));
        }

        output.push_str("\t\t}\n\t}\n}\n");
    }

    println!("{:?}", stc);
    println!("\n{}", output);
}

fn calculate_type(f: &Field) -> String {
    if !f.optional {
        return f.field_type.to_owned();
    }

    // not optional
    match f.initializer {
        Some(_) => f.field_type.to_owned(),
        None => format!("Option<{}>", f.field_type),
    }
}

fn calculate_constructor_parameters(stc: &Struct) -> String {
    let mut s = String::new();
    let mut f_first = true;

    for cp in stc.constructor_fields.iter() {
        if !f_first {
            s.push_str(", ");
        }
        s.push_str(&format!("{}: {}", cp.name, cp.field_type));
        f_first = false;
    }
    s
}

fn calculate_type_description_all_no(stc: &Struct) -> String {
    let mut s = String::new();

    let mut f_first = true;
    for f in &stc.extra_types {
        if !f_first {
            s.push_str(", ");
        }

        s.push_str(&f);
        f_first = false;
    }

    for _ in stc.fields.iter().filter(|f| f.optional == false) {
        if !f_first {
            s.push_str(", No");
        }

        f_first = false;
    }

    if s.is_empty() {
        "".to_owned()
    } else {
        format!("<{}>", s)
    }
}

fn calculate_type_description(stc: &Struct, builders_type_to_skip: &[String]) -> String {
    let mut s = String::new();

    let mut f_first = true;
    for f in &stc.extra_types {
        if !f_first {
            s.push_str(", ");
        }

        s.push_str(&f);
        f_first = false;
    }

    for f in stc
        .fields
        .iter()
        .filter(|f| f.optional == false)
        .filter(|f| {
            let bt = f.builder_type.clone().unwrap();
            !builders_type_to_skip.contains(&bt)
        }) {
        if !f_first {
            s.push_str(", ");
        }

        let bt = match f.builder_type {
            Some(ref a) => a,
            None => panic!(),
        };

        s.push_str(&bt);
        f_first = false;
    }

    if s.is_empty() {
        "".to_owned()
    } else {
        format!("<{}>", s)
    }
}

fn calculate_where(stc: &Struct) -> String {
    let mut s = String::new();

    for f in stc.fields.iter().filter(|f| !f.optional) {
        s.push_str(&format!(
            "\t{} : ToAssign,\n",
            f.clone().builder_type.unwrap()
        ));
    }

    if s.is_empty() {
        "".to_owned()
    } else {
        format!("where\n{}", s)
    }
}
