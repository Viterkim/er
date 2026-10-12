# What pattern should I use?

Most of your code uses the boring .er() everywhere, but things can get spicy, in any case you can always have all the 'er' stuff be internal, and just export normal errors with #[derive(Er)] for your consumer, and they never have to know about 'er' or its types, its boring normal structs/enums.

## Most common cases

### Inside your code

[.er(...)](examples.md#data-struct) builds the tree, keeping the original failure below your context.

### A public error for your library's users

An ordinary `#[derive(Er)]` type. [.er_into(...)](examples.md#public-error) can save diagnostics on it.

### Their trait on your type

When you choose the error type, use your own error or tree depending on what they accept. No Wrap needed just for this.

### Their trait on the whole tree

Generate [Wrap](macros.md#wrap), so you own the type receiving the implementation.

## Extra cases

### Their required error has a boxed Error slot

Box an ordinary error, or a [std_error Wrap](macros.md#wrap-with-std_error) to keep a live tree.

### That ordinary diagnostic needs a location

Put [#[er(src_location)]](examples.md#foreign-callbacks) on its location field.

### Their error has nowhere to keep your details

Return their classification and report your details separately.

### They require Error on that Wrap (you usually NEVER need this)

You usually never ever need this, you can usually always get away with just using wrap without error.

Use [std_error](macros.md#wrap-with-std_error), with `output = top` or `output = report`. Open its returned Result with `.er_tree()` before using `.er()`, or use `.er_wrap(...)` directly.
