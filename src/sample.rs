trait Dispatcher {
    fn dispatch(&self, closure: impl Fn) -> ();
}

struct Item<T: Dispatcher> {
    dispatcher: T,
}

impl Item {
    fn handle(&self, closure: impl Fn) {
        self.dispatcher.dispatch(closure);
    }
}

fn main(item: Item) {
    item.handle("adad");
}
