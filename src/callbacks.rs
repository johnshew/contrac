pub struct Dispatcher<'a> {
    callbacks: Vec<Box<dyn 'a + Fn()>>,
}

impl<'a> Dispatcher<'a> {
    pub fn new() -> Dispatcher<'a> {
        Dispatcher {
            callbacks: Vec::new(),
        }
    }
    pub fn add_callback<CB: 'a + Fn()>(&mut self, c: CB) {
        self.callbacks.push(Box::new(c));
    }
    pub fn invoke(&self) {
        for f in self.callbacks {
            (f)();
        }
    }
}
