# lispfmt

Lisp formatting as a library.

## Usage

- Define a type that implements `Atom`, such as a newtype around `&str`.
- Feed an iterator of `Token`s to `lispfmt::format`.

## Technical details

`lispfmt` is `no_std`-compatible, requiring only an allocator. Since it consumes
tokens and not syntax trees, it does not rely on recursion and can therefore
handle pathological inputs.
