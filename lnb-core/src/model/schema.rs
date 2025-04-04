use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

impl DescribedSchema {
    pub fn integer(name: impl Into<String>, description: impl Into<String>) -> DescribedSchema {
        DescribedSchema {
            name: name.into(),
            description: description.into(),
            field_type: DescribedSchemaType::Integer,
        }
    }

    pub fn float(name: impl Into<String>, description: impl Into<String>) -> DescribedSchema {
        DescribedSchema {
            name: name.into(),
            description: description.into(),
            field_type: DescribedSchemaType::Float,
        }
    }

    pub fn boolean(name: impl Into<String>, description: impl Into<String>) -> DescribedSchema {
        DescribedSchema {
            name: name.into(),
            description: description.into(),
            field_type: DescribedSchemaType::Boolean,
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
        }
    }

    pub fn string(name: impl Into<String>, description: impl Into<String>) -> DescribedSchema {
        DescribedSchema {
            name: name.into(),
            description: description.into(),
            field_type: DescribedSchemaType::String,
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
        }
    }
}
