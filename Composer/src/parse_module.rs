use super::*;

impl Composer {
    pub fn capitalize(&self, s: &str) -> String {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }

    pub fn get_macros(&self) -> String {
        format!(
            "use serde_json::Value;
use serde_derive::{{Serialize, Deserialize}};
use std::collections::HashMap;

macro_rules! make_input_struct {{
    (
        $x:ident,
        // list of derive macros
        [$($visibality:vis $element:ident : $ty:ty),*],
        // list of derive macros
        [$($der:ident),*]
) => {{
        #[derive($($der),*)]
        struct $x {{ $($visibality  $element: $ty),*}}
    }}
}}

macro_rules! make_main_struct {{
    (
        $name:ident,
        $input:ty,
        [$($der:ident),*],
        // list of attributes
        [$($key:ident : $val:expr),*]
) => {{
        #[derive($($der),*)]
        $(
            #[$key = $val]
        )*
        struct $name {{
            action_name: String,
            pub input: $input,
            pub output: Value,
        }}
        impl $name{{
            pub fn output(&self) -> Value {{
                self.output.clone()
            }}
        }}
    }}
}}

macro_rules! impl_new {{
    (
        $name:ident,
        $input:ident,
        []
    ) => {{
        impl $name{{
            pub fn new(action_name:String) -> Self{{
                Self{{
                    action_name,
                    input: $input{{
                        ..Default::default()
                    }},
                    ..Default::default()
                }}      
            }}
        }}
    }};
    (
        $name:ident,
        $input:ident,
        [$($element:ident : $ty:ty),*]
    ) => {{
        impl $name{{
            pub fn new($( $element: $ty),*, action_name:String) -> Self{{
                Self{{
                    action_name,
                    input: $input{{
                        $($element),*,
                        ..Default::default()
                    }},
                    ..Default::default()
                }}      
            }}
        }}
    }}
}}

macro_rules! impl_setter {{
    (
        $name:ty,
        [$($element:ident : $key:expr),*]
    ) => {{
        impl $name{{
            pub fn setter(&mut self, val: Value) {{
                $(
                let value = val.get($key).unwrap();
                self.input.$element = serde_json::from_value(value.clone()).unwrap();
                )*
            }}
        }}
    }}
}}"
        )
    }

    pub fn get_attributes(&self, task: &HashMap<String, String>) -> String {
        let mut attributes = String::from("[");

        for (i, (k, v)) in task.iter().enumerate() {
            attributes = format!("{attributes}{}:\"{}\"", k, v);

            attributes = if i != task.len() - 1 {
                format!("{attributes},")
            } else {
                format!("{attributes}]")
            }
        }

        attributes
    }

    fn get_kind(&self, kind: &str) -> Result<String, ErrorKind> {
        match kind.to_lowercase().as_str() {
            "openwhisk" => Ok(String::from("OpenWhisk")),
            "polkadot" => Ok(String::from("Polkadot")),
            _ => Err(ErrorKind::NotFound),
        }
    }

    pub fn get_custom_structs(&self) -> Vec<String> {
        let mut common_inputs = HashMap::<String, String>::new();
        let asd = self.workflows.borrow();

        let mut constructors = String::new();
        let mut input_structs = String::new();

        for (task_name, task) in self.workflows.borrow()[0].tasks.iter() {
            let task_name = self.capitalize(&task_name);

            let mut depend = Vec::<String>::new();
            let mut setter = Vec::<String>::new();

            for fields in task.depend_on.values() {
                let x = fields.iter().next().unwrap();
                depend.push(String::from(x.0));

                setter.push(format!("{}:\"{}\"", x.0, x.1));
            }

            let mut input = format!(
                "make_input_struct!(
    {task_name}Input,
    ["
            );

            let mut new = Vec::<String>::new();

            for (i, field) in task.input_args.iter().enumerate() {
                input = format!("{input}{}:{}", field.0, field.1);

                if i != task.input_args.len() - 1 {
                    input = format!("{input},");
                } else {
                    input =
                        format!("{input}],\n\t[Debug, Clone, Default, Serialize, Deserialize]);");
                }

                if let Err(_) = depend.binary_search(field.0) {
                    common_inputs.insert(String::from(field.0), String::from(field.1));
                    new.push(format!("{}:{}", field.0, field.1));
                }
            }

            input_structs = format!(
                "{input_structs}
{input}
make_main_struct!(
    {task_name},
    {task_name}Input,
    [Debug, Clone, Default, Serialize, Deserialize, {}],
    {}
);
impl_new!(
    {task_name},
    {task_name}Input,
    [{}]
);
impl_setter!({task_name}, [{}]);
",
                self.get_kind(&task.kind).unwrap(),
                self.get_attributes(&task.attributes),
                new.join(","),
                setter.join(",")
            );

            constructors = if new.len() == 0 {
                format!(
                    "{constructors}\tlet {} = {}::new(\"{}\".to_string());\n",
                    task_name.to_lowercase(),
                    task_name,
                    task.action_name.clone()
                )
            } else {
                let commons: Vec<String> = new
                    .iter()
                    .map(|x| format!("input.{}", x.split(":").collect::<Vec<&str>>()[0]))
                    .collect();

                format!(
                    "{constructors}\tlet {} = {}::new({}, \"{}\".to_string());\n",
                    task_name.to_lowercase(),
                    task_name,
                    commons.join(","),
                    task.action_name.clone()
                )
            };

            constructors = format!(
                "{constructors}\tlet {}_index = workflow::add_node(Box::new({}));\n",
                task_name.to_lowercase(),
                task_name.to_lowercase()
            );
        }

        let mut input = String::from("\nmake_input_struct!(\n\tInput,\n\t[");

        for (i, field) in common_inputs.iter().enumerate() {
            input = format!("{input}{}:{}", field.0, field.1);

            if i != common_inputs.len() - 1 {
                input = format!("{input},");
            } else {
                input = format!("{input}],\n\t[Debug, Clone, Default, Serialize, Deserialize]);");
            }
        }

        input_structs = format!("{input_structs}\n{input}");
        vec![input_structs, constructors]
    }

    pub fn get_workflow_execute_code(&self) -> String {
        let mut execute_code = format!("\tlet result = workflow\n\t\t.int()?\n");

        let mut add_edges_code = String::from("\tworkflow.add_edges(&[\n");
        let flow: Vec<String> = self.get_flow();

        for i in 0..flow.len() - 1 {
            add_edges_code = format!(
                "{add_edges_code}\t\t({}_index, {}_index),\n",
                flow[i].to_lowercase(),
                flow[i + 1].to_lowercase()
            );

            execute_code = if i + 1 == flow.len() - 1 {
                match self
                    .workflows.borrow()[0].tasks
                    .get(&flow[i + 1])
                    .unwrap()
                    .depend_on
                    .len()
                {
                    0 | 1 => {
                        format!(
                            "{execute_code}\t\t.term(Some({}_index))?;\n",
                            flow[i + 1].to_lowercase()
                        )
                    }

                    _ => {
                        format!(
                            "{execute_code}\t\t.pipe({}_index)?\n\t\t.term(None)?;\n",
                            flow[i + 1].to_lowercase()
                        )
                    }
                }
            } else {
                format!(
                    "{execute_code}\t\t.pipe({}_index)?\n",
                    flow[i + 1].to_lowercase()
                )
            };
        }

        add_edges_code = format!("{add_edges_code}\t]);\n\n{execute_code}");

        add_edges_code
    }

    pub fn generate_main_file_code(&self) -> String {
        let structs = self.get_custom_structs();

        let main_file = format!(
            "{}
{}

#[allow(dead_code, unused)]
pub fn main(args: Value) -> Result<Value, String> {{
    const LIMIT: usize = {};
    let mut workflow = WorkflowGraph::new(LIMIT);
    let input: Input = serde_json::from_value(args).map_err(|e| e.to_string())?;

{}
{}
    let result = serde_json::to_value(result).unwrap();
    Ok(result)
}}
",
            self.get_macros(),
            structs[0],
            self.workflows.borrow()[0].tasks.len(),
            structs[1],
            self.get_workflow_execute_code()
        );

        main_file
    }
}
