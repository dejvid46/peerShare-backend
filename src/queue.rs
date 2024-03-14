#[derive(Debug)]
pub struct Queue {
    empty_spaces: Vec<usize>,
    pointer: usize,
}

impl Queue {
    pub fn new(lenght: usize) -> Queue {
        Queue {
            empty_spaces: (0..lenght).collect(),
            pointer: 0,
        }
    }
}

impl Queue {
    pub fn reserve(&mut self) -> Option<&usize> {
        let res = self.empty_spaces.get(self.pointer);

        if res.is_some() {
            self.pointer += 1;
        }

        //println!("Pointer is: {:?}, room id is {:?}", self.pointer, res);

        res
    }
}

impl Queue {
    pub fn refund(&mut self, id: &usize) {
        self.pointer -= 1;
        self.empty_spaces[self.pointer] = id.to_owned();
        //println!("Pointer is: {:?}, room return id is {:?}", self.pointer, id);
    }
}
