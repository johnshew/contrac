pub struct Callbacks<'b, T, R: Copy + Clone> {
    callbacks: Vec<Box<dyn for<'a> FnMut(&'a mut T) -> R + 'b>>,
}


// use anyhow::{Context, Result};

impl<'b, T, R: Copy + Clone> Callbacks<'b, T, R> {
    pub fn new() -> Callbacks<'b, T, R> {
        Callbacks {
            callbacks: Vec::new(),
        }
    }
    pub fn add<F>(&mut self, callback: F)
    where
        F: for<'a> FnMut(&'a mut T) -> R + 'b,
    {
        self.callbacks.push(Box::new(callback));
    }

    pub fn invoke(&mut self, t: &mut T) -> Vec<R> {
        let mut results = Vec::<R>::new();
        for f in &mut self.callbacks {
            let result = (f)(t);
            results.push(result);
        }
        results
    }
}

pub struct NotifyCallback<'a, T> {
    callbacks: Vec<Box<dyn for<'b> FnMut(&'b mut T) -> () +'a>>,
}

impl<'a, 'b, T: 'b> NotifyCallback<'a, T> {
    pub fn new() -> NotifyCallback<'a, T> {
        NotifyCallback {
            callbacks: Vec::new(),
        }
    }
    pub fn add< F>(&mut self, callback: F)
    where
        F: for<'z> FnMut(&'z mut T) -> () +'a,
    {
        self.callbacks.push(Box::new(callback));
    }

    pub fn invoke(&mut  self, t: &mut T) -> () {
        for f in &mut self.callbacks {
           let result = (f)( t);
        }
        ()
    }
}

type NotifyCallback2<'a, T> = Callbacks<'a, T, ()>;

#[cfg(test)]
#[test]
fn does_it_work() {

    #[derive(Debug, Copy, Clone)]
    struct Inner {
        value: i32,
        count: u32,
    }
    
    struct Outer<'a> {
        data: Inner,
        pub value_modified: NotifyCallback2<'a, Inner>,
    }

    impl<'a> Outer<'a> {
        pub fn new() -> Outer<'a> {
            Outer {
                data: Inner { value: 0, count: 0 },
                value_modified: NotifyCallback2::new(),
            }
        }
        pub fn set_value(&mut self, v: i32) {
            self.data.value = v;
            self.value_modified.invoke(&mut self.data);
        }
        pub fn get_value(&self) -> Inner {
           self.data
        }
    }
    let increment = 1; // lifetime needs to be longer than test 
    let mut test = Outer::new();
    test.value_modified.add(|_| {
        println!("Hello ");
    });
    test.value_modified.add(|_| {
        println!("World!");
    });
    test.value_modified.add(|stuff| {
        stuff.count += increment; // closure references checked by rust
        println!("Modified: {:?}", stuff);
    });

    println!("Initial: {:?}", test.get_value());
    test.set_value(10);
    let test = test;
    let v = test.get_value();
    println!("Final: {:?}", v);
}