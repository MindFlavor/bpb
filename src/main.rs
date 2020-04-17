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
    pub derive: Option<String>,
    pub uses: Vec<String>,
    pub inline: Option<bool>,
    pub extra_types: Vec<String>,
    pub extra_wheres: Vec<String>,
    pub constructor_fields: Vec<ConstructorField>,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum YesNo {
    Yes,
    No,
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
            .for_each(|bt| abt.push(bt.builder_type.clone().unwrap()));
        abt
    };

    let mut output = String::new();

    let mut regardless = String::new();

    // dump uses
    if !stc.uses.is_empty() {
        stc.uses.iter().for_each(|u| {
            if u.chars().last().unwrap() == ';' {
                output.push_str(&format!("use {}\n", u))
            } else {
                output.push_str(&format!("use {};\n", u))
            }
        });

        output.push_str("\n");
    }

    // dump derives, if any
    if let Some(ref derive) = stc.derive {
        output.push_str(&format!("#[derive({})]\n", derive));
    }

    // create the struct
    {
        output.push_str(&format!(
            "pub struct {}{}\n{} {{\n",
            stc.name,
            calculate_type_description(&stc, &[], None),
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
            calculate_type_description(&stc, &all_builder_types, None),
            stc.name,
            calculate_type_description_all(&stc, YesNo::No),
            calculate_where(&stc, &all_builder_types)
        ));

        if stc.inline() {
            output.push_str("#[inline]\n");
        }

        output.push_str(&format!(
            "\t pub(crate) fn new({}) -> {}{} {{\n\t\t{} {{\n",
            calculate_constructor_parameters(&stc),
            stc.name,
            calculate_type_description_all(&stc, YesNo::No),
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
                calculate_type_description(&stc, &[], None),
                t,
                stc.name,
                calculate_type_description(&stc, &[], None),
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
            //println!("\n\nct ==> {:?}", ct);
            //println!("regardless  ==> {}", regardless);
            //regardless.push_str(&format!("{}{{\n", &calculate_where(&stc, &[])));
            if stc.inline() {
                output.push_str("#[inline]\n");
            }
            regardless.push_str(&format!(
                "\tfn {}(&self) -> {} {{\n\t\tself.{}\n\t}}\n\n",
                ct.name, ct.field_type, ct.name
            ));
            //println!("regardless  ==> {}", regardless);
        }
    }

    // get mandatory no traits methods
    {
        output.push_str("\n//get mandatory no traits methods\n");
        for tm in stc
            .fields
            .iter()
            .filter(|tm| tm.trait_get.is_none() && tm.optional == false)
        {
            let bt = match tm.clone().builder_type {
                Some(bt) => vec![bt],
                None => Vec::new(),
            };

            output.push_str(&format!(
                "impl{} {}{}\n",
                calculate_type_description(&stc, &bt[..], None),
                stc.name,
                calculate_type_description(&stc, &bt[..], Some(YesNo::Yes)),
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

    // set mandatory no trait methods
    output.push_str("\n//set mandatory no traits methods\n");
    {
        for tm in stc
            .fields
            .iter()
            .filter(|tm| tm.trait_get.is_none() && tm.optional == false)
        {
            let bt = match tm.clone().builder_type {
                Some(bt) => vec![bt],
                None => Vec::new(),
            };

            output.push_str(&format!(
                "impl{} {}{}\n",
                calculate_type_description(&stc, &bt[..], None),
                stc.name,
                calculate_type_description(&stc, &bt[..], Some(YesNo::No)),
            ));

            output.push_str(&format!("{}\n{{\n", calculate_where(&stc, &bt[..])));

            let return_type = format!(
                "{}{}",
                stc.name,
                calculate_type_description(&stc, &bt[..], Some(YesNo::Yes))
            );

            if stc.inline() {
                output.push_str("#[inline]\n");
            }
            output.push_str(&format!(
                "\tfn with_{}(self, {}: {}) -> {} {{\n",
                tm.name, tm.name, tm.field_type, return_type
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
                calculate_type_description(&stc, &bt[..], None),
                tg,
                stc.name,
                calculate_type_description(&stc, &bt[..], Some(YesNo::Yes)),
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

            output.push_str(&format!(
                "impl{} {} for {}{}\n",
                calculate_type_description(&stc, &bt[..], None),
                tg,
                stc.name,
                calculate_type_description(&stc, &bt[..], Some(YesNo::No)),
            ));

            output.push_str(&format!("{}\n{{\n", calculate_where(&stc, &bt[..])));

            output.push_str(&format!(
                "\ttype O = {}{};\n\n",
                stc.name,
                calculate_type_description(&stc, &bt[..], Some(YesNo::Yes))
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
                regardless.push_str("#[inline]\n");
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
                regardless.push_str("#[inline]\n");
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
                regardless.push_str(&format!("\t\t\t\tp_{}: self.tp_{},\n", f.name, f.name));
            }

            for f in &stc.fields {
                if f.name == tm.name {
                    regardless.push_str(&format!("\t\t\t\t{}: Some({}),\n", f.name, f.name));
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
            calculate_type_description(&stc, &[], None),
            stc.name,
            calculate_type_description(&stc, &[], None)
        ));

        output.push_str(&format!("{}\n", calculate_where(&stc, &[])));

        output.push_str(&format!("{{\n{}\n", &regardless));
        output.push_str("}\n");
    }

    // print final
    {
        output.push_str("\n// methods callable only when every mandatory field has been filled\n");
        output.push_str(&format!(
            "impl{} {}{}\n",
            calculate_type_description(&stc, &all_builder_types, None),
            stc.name,
            calculate_type_description_all(&stc, YesNo::Yes),
        ));

        output.push_str(&format!("{}\n", calculate_where(&stc, &all_builder_types)));

        //output.push_str(&format!("{{\n{}\n", &regardless));
        output.push_str("{\n");
        output.push_str("}\n");
    }

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

fn calculate_type_description_all(stc: &Struct, yes_no: YesNo) -> String {
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
            match yes_no {
                YesNo::Yes => s.push_str(", Yes"),
                YesNo::No => s.push_str(", No"),
            }
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
    replace_with: Option<YesNo>,
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
            if let Some(yes_no) = replace_with {
                if !f_first {
                    s.push_str(", ");
                }

                match yes_no {
                    YesNo::Yes => s.push_str("Yes"),
                    YesNo::No => s.push_str("No"),
                }
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
