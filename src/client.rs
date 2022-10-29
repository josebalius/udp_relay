pub struct Client {}

impl Client {
    pub fn new() -> Self {
        Self {}
    }

    async fn connect(&self) {
        let (a, b) = tokio::join!(self.a(), self.b());
    }

    async fn a(&mut self) {}

    async fn b(&mut self) {}
}
