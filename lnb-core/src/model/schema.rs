use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum DescribedSchemaType {
    Integer,
    Float,
    Boolean,
    String,
    Array(Box<DescribedSchema>),
    Object(Vec<DescribedSchema>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescribedSchema {
    pub name: String,
    pub description: String,
    pub field_type: DescribedSchemaType,
    pub optional: bool,
}

impl DescribedSchema {
    pub fn integer(name: impl Into<String>, description: impl Into<String>) -> DescribedSchema {
        DescribedSchema {
            name: name.into(),
            description: description.into(),
            field_type: DescribedSchemaType::Integer,
            optional: false,
        }
    }

    pub fn float(name: impl Into<String>, description: impl Into<String>) -> DescribedSchema {
        DescribedSchema {
            name: name.into(),
            description: description.into(),
            field_type: DescribedSchemaType::Float,
            optional: false,
        }
    }

    pub fn boolean(name: impl Into<String>, description: impl Into<String>) -> DescribedSchema {
        DescribedSchema {
            name: name.into(),
            description: description.into(),
            field_type: DescribedSchemaType::Boolean,
            optional: false,
        }
    }

    pub fn array(
        name: impl Into<String>,
        description: impl Into<String>,
        item_schema: DescribedSchema,
    ) -> DescribedSchema {
        DescribedSchema {
            name: name.into(),
            description: description.into(),
            field_type: DescribedSchemaType::Array(Box::new(item_schema)),
            optional: false,
        }
    }

    pub fn string(name: impl Into<String>, description: impl Into<String>) -> DescribedSchema {
        DescribedSchema {
            name: name.into(),
            description: description.into(),
            field_type: DescribedSchemaType::String,
            optional: false,
        }
    }

    pub fn object(
        name: impl Into<String>,
        description: impl Into<String>,
        fields: impl IntoIterator<Item = DescribedSchema>,
    ) -> DescribedSchema {
        DescribedSchema {
            name: name.into(),
            description: description.into(),
            field_type: DescribedSchemaType::Object(fields.into_iter().collect()),
            optional: false,
        }
    }

    pub fn as_nullable(mut self) -> DescribedSchema {
        self.optional = true;
        self
    }
}
