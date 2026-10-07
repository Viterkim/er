use crate::{ErContextExt, ErInput, ErIteratorExt, ErMake, ErResult, aggregate};
use alloc::vec::Vec;
use core::error::Error;

impl<I, T, E, Mode, Input> ErIteratorExt<Mode, Input> for I
where
    I: Iterator<Item = Result<T, E>>,
    E: ErInput<Input>,
{
    type Ok = T;

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_collect<C, A>(self, error: impl ErMake<A, Mode>) -> ErResult<C, A>
    where
        C: FromIterator<T>,
        A: Error + 'static,
    {
        self.collect::<Result<C, E>>().er(error)
    }

    #[cfg_attr(feature = "src_locations", track_caller)]
    fn er_collect_all<C, A>(self, error: impl ErMake<A, Mode>) -> ErResult<C, A>
    where
        C: FromIterator<T>,
        A: Error + 'static,
    {
        let mut values = Vec::new();
        aggregate::collect(
            || error.er_make(),
            self.map(|result| result.map(|value| values.push(value))),
        )?;
        Ok(values.into_iter().collect())
    }
}
