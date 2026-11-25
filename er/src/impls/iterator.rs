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
        let mut remaining = self.fuse();
        let mut failure = None;
        let values = remaining
            .by_ref()
            .map_while(|result| match result {
                Ok(value) => Some(value),
                Err(error) => {
                    failure = Some(error);
                    None
                }
            })
            .fuse()
            .collect();

        match failure.or_else(|| remaining.find_map(Result::err)) {
            Some(failure) => {
                drop(values);
                Err(failure).er(error)
            }
            None => Ok(values),
        }
    }

    #[track_caller]
    fn er_collect_all<C, A>(self, error: impl ErMake<A, Mode>) -> ErResult<C, A>
    where
        C: FromIterator<T>,
        A: Error + 'static,
    {
        // Growing from empty took twice as long for 64 successes. The room gets reserved at the first success, so a
        // first item that fails reserves nothing, and capped, so a million items don't reserve a million up front.
        let room = self.size_hint().0.min(1024);
        let mut values = Some(Vec::new());
        let failures = aggregate::gather(self.map(|result| match result {
            Ok(value) => {
                if let Some(values) = &mut values {
                    if values.is_empty() {
                        values.reserve_exact(room);
                    }
                    values.push(value);
                }
                Ok(())
            }
            Err(error) => {
                values = None;
                Err(error)
            }
        }));

        match values {
            Some(values) => {
                drop(error);
                Ok(values.into_iter().collect())
            }
            None => Err(failures.into_tree(error.er_make())),
        }
    }
}
