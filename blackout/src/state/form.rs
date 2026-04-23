#[derive(Clone, Debug)]
pub struct FormState {
    pub fields: Vec<String>,
    pub current_index: usize,
    pub obscure_inputs: bool,
    pub is_password: Vec<bool>,
}

impl FormState {
    pub fn new() -> Self {
        Self {
            fields: vec![String::new(), String::new(), String::new()],
            current_index: 0,
            obscure_inputs: true,
            is_password: vec![false, false, false],
        }
    }

    pub fn clear(&mut self) {
        for field in self.fields.iter_mut() {
            field.clear();
        }
        self.current_index = 0;
    }
}
