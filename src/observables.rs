
pub trait IObserver {
    fn update(&self);
    // fn update_value<T>(&self, value: T);
}

pub trait ISubject<'a> {
    fn attach(&mut self, observer: &'a dyn IObserver) {
        self.observers_mut().push(observer);
    }
    fn detach(&mut self, observer: &'a dyn IObserver) {
        if let Some(idx) = self.observers().iter().position(|x|  (*x) as *const dyn IObserver == observer as *const dyn IObserver) {
            self.observers_mut().remove(idx);
        }
    }
    fn notify_observers(& self) {
        for item in self.observers().iter() {
            item.update();
        }
    }
    // fn notify_obsevers_value<T> (&self, value: T) {
    //     for item in self.observers().iter() {
    //         item.update_value(value);
    //     }
    // }
    fn observers(&self) -> & Vec<&'a dyn IObserver>;
    fn observers_mut(&mut self) -> & mut Vec<&'a dyn IObserver>;
}

pub struct Subject<'a> {
    observers_impl: Vec<&'a dyn IObserver>,
}


impl Subject<'_> {
    pub fn new() -> Self {
        Self {
            observers_impl: Vec::new(),
        }
    }
}

// impl<'a> Subject<'a> {
//     pub fn new() -> Subject<'a> {
//         Subject {
//             observers_impl: Vec::new(),
//         }
//     }
// }

impl<'a> ISubject<'a> for Subject<'a> {
    fn observers(& self) -> & Vec<&'a dyn IObserver> { & self.observers_impl }
    fn observers_mut(&mut self) -> & mut Vec<&'a dyn IObserver> { & mut self.observers_impl }
}