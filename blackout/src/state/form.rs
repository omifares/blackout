#[derive(Clone, Debug)]
pub struct FormState {
    pub fields: Vec<String>,
    pub current_field: usize,
    pub cursor_index: usize,
    pub obscure_inputs: bool,
}

impl FormState {
    pub fn new() -> Self {
        Self {
            fields: vec![String::new(), String::new(), String::new()],
            current_field: 0,
            cursor_index: 0,
            obscure_inputs: true,
        }
    }

    pub fn clear(&mut self) {
        for field in self.fields.iter_mut() {
            field.clear();
        }
        self.current_field = 0;
        self.cursor_index = 0;
    }
}
