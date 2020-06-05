use async_std::stream;
use iced_futures::futures;
use iced_futures::subscription::Recipe;

pub struct Every(pub std::time::Duration);

impl<H, E> Recipe<H, E> for Every
where
    H: std::hash::Hasher,
{
    type Output = ();

    fn hash(&self, state: &mut H) {
        use std::hash::Hash;

        std::any::TypeId::of::<Self>().hash(state);
        self.0.hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: futures::stream::BoxStream<'static, E>,
    ) -> futures::stream::BoxStream<'static, Self::Output> {
        Box::pin(stream::interval(self.0))
    }
}
