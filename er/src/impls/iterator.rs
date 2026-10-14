use crate::{ErContextExt, ErInput, ErIteratorExt, ErMake, ErResult, aggregate};
use alloc::vec::Vec;
use core::error::Error;

impl<I, T, E, Mode, Input> ErIteratorExt<Mode, Input> for I
where
    I: Iterator<Item = Result<T, E>>,
    E: ErInput<Input>,
{
    type Ok = T;

    #[track_caller]
    fn er_collect<C, A>(self, error: impl ErMake<A, Mode>) -> ErResult<C, A>
    where
        C: FromIterator<T>,
        A: Error + 'static,
    {
        self.collect::<Result<C, E>>().er(error)
    }

    #[track_caller]
    fn er_collect_all<C, A>(self, error: impl ErMake<A, Mode>) -> ErResult<C, A>
    where
        C: FromIterator<T>,
        A: Error + 'static,
    {
        let mut values = Vec::new();
        let failures = aggregate::gather(self.map(|result| result.map(|value| values.push(value))));

        if failures.is_empty() {
            drop(error);
            Ok(values.into_iter().collect())
        } else {
            drop(values);
            Err(failures.into_tree(error.er_make()))
        }
    }
}
