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
    pub inline: Option<bool>,
    pub extra_types: Vec<String>,
    pub extra_wheres: Vec<String>,
    pub constructor_fields: Vec<ConstructorField>,
    pub fields: Vec<Field>,
}

impl Struct {
    pub fn inline(&self) -> bool {
        if let Some(i) = self.inline {
            i
        } else {
            false
        }
    }
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

    let mut regardless = String::new();

    // create the struct
    {
        output.push_str(&format!(
            "pub struct {}{}\n{} {{\n",
            stc.name,
            calculate_type_description(&stc, &[], false),
            calculate_where(&stc, &[])
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
            "impl{} {}{} {} {{\n",
            calculate_type_description(&stc, &all_builder_types, false),
            stc.name,
            calculate_type_description_all_no(&stc),
            calculate_where(&stc, &all_builder_types)
        ));

        if stc.inline() {
            output.push_str("#[inline]\n");
        }

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
            match f.initializer {
                Some(ref initializer) => {
                    output.push_str(&format!("\t\t\t{}: {},\n", f.name, initializer))
                }
                None => output.push_str(&format!("\t\t\t{}: None,\n", f.name)),
            };
        }

        output.push_str("\t\t}\n\t}\n}\n\n");
    }

    // constructor types getter
    {
        // first the one with trait
        for ct in stc
            .constructor_fields
            .iter()
            .filter(|ct| ct.trait_get.is_some())
        {
            let t = ct.trait_get.clone().unwrap();
            output.push_str(&format!(
                "impl{} {} for {}{}\n",
                calculate_type_description(&stc, &[], false),
                t,
                stc.name,
                calculate_type_description(&stc, &[], false),
            ));

            output.push_str(&format!("{}{{\n", &calculate_where(&stc, &[])));

            if stc.inline() {
                output.push_str("#[inline]\n");
            }
            output.push_str(&format!(
                "\tfn {}(&self) -> {} {{\n\t\tself.{}\n\t}}\n\n",
                ct.name, ct.field_type, ct.name
            ));

            output.push_str("}\n\n");
        }

        // now the ones without trait
        for ct in stc
            .constructor_fields
            .iter()
            .filter(|ct| ct.trait_get.is_none())
        {
            regardless.push_str(&format!("{}{{\n", &calculate_where(&stc, &[])));
            if stc.inline() {
                output.push_str("#[inline]\n");
            }
            regardless.push_str(&format!(
                "\tfn {}(&self) -> {} {{\n\t\tself.{}\n\t}}\n\n",
                ct.name, ct.field_type, ct.name
            ));
        }
    }

    // get traits methods
    {
        for tm in stc.fields.iter().filter(|tm| tm.trait_get.is_some()) {
            let bt = match tm.clone().builder_type {
                Some(bt) => vec![bt],
                None => Vec::new(),
            };
            let tg = tm.trait_get.clone().unwrap();

            output.push_str(&format!(
                "impl{} {} for {}{}\n",
                calculate_type_description(&stc, &bt[..], false),
                tg,
                stc.name,
                calculate_type_description(&stc, &bt[..], true),
            ));

            output.push_str(&format!("{}\n{{\n", calculate_where(&stc, &bt[..])));

            if stc.inline() {
                output.push_str("#[inline]\n");
            }
            output.push_str(&format!("\tfn {}(&self) -> ", tm.name));

            if tm.optional && tm.initializer.is_none() {
                output.push_str(&format!("Option<{}> {{\n", tm.field_type));
            } else {
                output.push_str(&format!("{} {{\n", tm.field_type));
            }

            output.push_str(&format!("\t\tself.{}", tm.name));
            if !tm.optional && tm.initializer.is_none() {
                output.push_str(".unwrap()\n\t}\n}\n\n");
            } else {
                output.push_str("\n\t}\n}\n\n");
            }
        }
    }

    // set trait methods
    {
        for tm in stc.fields.iter().filter(|tm| tm.trait_get.is_some()) {
            let bt = match tm.clone().builder_type {
                Some(bt) => vec![bt],
                None => Vec::new(),
            };
            let tg = tm.trait_set.clone().unwrap();

            let full_type_desc = calculate_type_description(&stc, &[], false);

            output.push_str(&format!(
                "impl{} {} for {}{}\n",
                full_type_desc, tg, stc.name, full_type_desc
            ));
            output.push_str(&format!("{}\n{{\n", calculate_where(&stc, &[])));

            output.push_str(&format!(
                "\ttype O = {}{};\n\n",
                stc.name,
                calculate_type_description(&stc, &bt[..], true)
            ));

            if stc.inline() {
                output.push_str("#[inline]\n");
            }
            output.push_str(&format!(
                "\tfn with_{}(self, {}: {}) -> Self::O {{\n",
                tm.name, tm.name, tm.field_type
            ));

            output.push_str(&format!("\t\t{} {{\n", stc.name));

            // constructor types
            for t in stc.constructor_fields.iter() {
                output.push_str(&format!("\t\t\t\t{}: self.{},\n", t.name, t.name));
            }

            // phantom types
            for f in stc.fields.iter().filter(|f| !f.optional) {
                output.push_str(&format!("\t\t\t\tp_{}: PhantomData{{}},\n", f.name,));
            }

            for f in &stc.fields {
                if f.name == tm.name {
                    if tm.initializer.is_some() {
                        output.push_str(&format!("\t\t\t\t{},\n", f.name));
                    } else {
                        output.push_str(&format!("\t\t\t\t{}: Some({}),\n", f.name, f.name));
                    }
                } else {
                    output.push_str(&format!("\t\t\t\t{}: self.{},\n", f.name, f.name));
                }
            }

            output.push_str("\t\t}\n\t}\n}\n\n");
        }
    }

    // get optional without traits
    {
        for tm in stc
            .fields
            .iter()
            .filter(|tm| tm.optional && tm.trait_get.is_none())
        {
            if stc.inline() {
                output.push_str("#[inline]\n");
            }
            regardless.push_str(&format!(
                "\tpub fn {}(&self) -> Option<{}> {{\n",
                tm.name, tm.field_type
            ));

            regardless.push_str(&format!("\t\tself.{}\n", tm.name));
            regardless.push_str("\t}\n\n");
        }
    }

    // set optional without traits
    {
        for tm in stc
            .fields
            .iter()
            .filter(|tm| tm.optional && tm.trait_get.is_none())
        {
            if stc.inline() {
                output.push_str("#[inline]\n");
            }
            regardless.push_str(&format!(
                "\tfn with_{}(self, {}: {}) -> Self {{\n",
                tm.name, tm.name, tm.field_type
            ));

            regardless.push_str(&format!("\t\t{} {{\n", stc.name));

            // constructor types
            for t in stc.constructor_fields.iter() {
                regardless.push_str(&format!("\t\t\t\t{}: self.{},\n", t.name, t.name));
            }

            // phantom types
            for f in stc.fields.iter().filter(|f| !f.optional) {
                regardless.push_str(&format!("\t\t\t\tp_{}: PhantomData{{}},\n", f.name,));
            }

            for f in &stc.fields {
                if f.name == tm.name {
                    regardless.push_str(&format!("\t\t\t\t{},\n", f.name));
                } else {
                    regardless.push_str(&format!("\t\t\t\t{}: self.{},\n", f.name, f.name));
                }
            }

            regardless.push_str("\t\t}\n\t}\n\n");
        }
    }

    // print regardless
    {
        output.push_str("// methods callable regardless\n");
        output.push_str(&format!(
            "impl{} {}{}\n",
            calculate_type_description(&stc, &[], false),
            stc.name,
            calculate_type_description(&stc, &[], false)
        ));

        output.push_str(&format!("{}\n", calculate_where(&stc, &[])));

        output.push_str(&format!("{{\n{}\n", &regardless));
        output.push_str("}\n");
    }

    //println!("{:?}", stc);
    println!("\n{}", output);
}

fn calculate_type(f: &Field) -> String {
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

fn calculate_type_description(
    stc: &Struct,
    builders_type_to_skip: &[String],
    f_replace_with_yes: bool,
) -> String {
    let mut s = String::new();

    let mut f_first = true;
    for f in &stc.extra_types {
        if !f_first {
            s.push_str(", ");
        }

        s.push_str(&f);
        f_first = false;
    }

    for f in stc.fields.iter().filter(|f| f.optional == false) {
        let bt = f.builder_type.clone().unwrap();
        if builders_type_to_skip.contains(&bt) {
            if f_replace_with_yes {
                if !f_first {
                    s.push_str(", ");
                }

                s.push_str("Yes");
                f_first = false;
            }
        } else {
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
    }

    if s.is_empty() {
        "".to_owned()
    } else {
        format!("<{}>", s)
    }
}

fn calculate_where(stc: &Struct, builders_type_to_skip: &[String]) -> String {
    let mut s = String::new();

    for f in stc.fields.iter().filter(|f| !f.optional).filter(|f| {
        let bt = f.builder_type.clone().unwrap();
        !builders_type_to_skip.contains(&bt)
    }) {
        s.push_str(&format!(
            "\t{} : ToAssign,\n",
            f.clone().builder_type.unwrap()
        ));
    }

    for ew in stc.extra_wheres.iter() {
        s.push_str(&format!("\t{},\n", ew));
    }

    if s.is_empty() {
        "".to_owned()
    } else {
        format!("where\n{}", s)
    }
}
