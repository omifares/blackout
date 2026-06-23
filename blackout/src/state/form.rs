use crate::state::{FieldConfig, FieldValue};

#[derive(Clone, Debug, PartialEq)]
pub struct FormState {
    pub fields: Vec<FieldConfig>,
    pub current_field: usize,
    pub cursor_index: usize,
    pub obscure_inputs: bool,
}

impl FormState {
    pub fn new() -> Self {
        Self {
            fields: vec![],
            current_field: 0,
            cursor_index: 0,
            obscure_inputs: true,
        }
    }

    pub fn clear(&mut self) {
        for field in self.fields.iter_mut() {
            field.value = FieldValue::Text(String::new());
        }
        self.current_field = 0;
        self.cursor_index = 0;
    }
}
