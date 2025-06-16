use super::*;

#[derive(Default, Debug)]
pub struct Output {
    pub name: String,
    pub amount_0: u128,
    pub amount_1: u128,
    pub path: Vec<Address>,
    pub to: Address,
    pub deadline: u128,
}

impl Output {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn display(&self, names: &[&str]) {
        if names.len() == 1 {
            println!(
                "name: {}\n{}: {}\npath: {:?}\nto: {:?}\ndeadline: {}",
                self.name, names[0], self.amount_0, self.path, self.to, self.deadline,
            );
        } else {
            println!(
                "name: {}\n{}: {}\n{}: {}\npath: {:?}\nto: {:?}\ndeadline: {}",
                self.name,
                names[0],
                self.amount_0,
                names[1],
                self.amount_1,
                self.path,
                self.to,
                self.deadline,
            );
        }
    }
}
