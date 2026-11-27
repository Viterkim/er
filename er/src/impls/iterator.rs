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
    #[inline]
    fn er_collect_all<C, A>(self, error: impl ErMake<A, Mode>) -> ErResult<C, A>
    where
        C: FromIterator<T>,
        A: Error + 'static,
    {
        // A large iterator may fail on its first item. Reserve on the first success.
        let room = self.size_hint().0.min(1024);
        let mut remaining = self;
        let mut values = Vec::new();

        while let Some(result) = remaining.next() {
            match result {
                Ok(value) => {
                    if values.is_empty() {
                        values.reserve_exact(room);
                    }
                    values.push(value);
                }
                Err(source) => {
                    drop(values);
                    let mut nodes = alloc::vec![source.into_er_input()];
                    aggregate::append(&mut nodes, remaining);

                    return Err(aggregate::tree_from_nodes(nodes, error.er_make()));
                }
            }
        }

        drop(remaining);
        drop(error);
        Ok(values.into_iter().collect())
    }
}
