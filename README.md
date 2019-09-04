`dbase-parser`: A dbaseIII/IV/foxpro iterative parser for rust
========================================================

This crate provides the ability to read and parse `dbf` files, 
yielding typed objects as it goes. It is efficient, versatile, 
well-tested and, above all, maintained.

The motivation behind this is documented 
[here](https://stackoverflow.com/a/57769207/2167834). In particular, 
no parsing crate supported the memo (`M`) type, let alone the two 
variants of it.

[![Build Status](https://travis-ci.org/srenauld/dbase.svg?branch=master)](https://travis-ci.org/srenauld/dbase)
[![Coverage Status](https://coveralls.io/repos/github/srenauld/dbase/badge.svg?branch=coveralls-fix)](https://coveralls.io/github/srenauld/dbase?branch=coveralls-fix)

# A disclaimer on versioning

As the crate is still under 
active development, versions may change relatively fast. Until we're 
at 1.x, consider the public API to be unstable.

In particular, the `FieldValue` enum may gain additional types as 
different DBF file formats surface.

# Installation

Add `dbase_parser` to your dependencies.

# Usage

Opening a file and streaming rows is a simple set of operations, as 
shown below:

    extern crate dbase_parser;
    use dbase_parser::{FieldValue, open};

    let dbase_file = open("data.dbf");
    let amount:f64 = dbase_file
      .map(|db| {
        db.into_iter().fold(0.0, |current, record| {
          record.get("amount").map(|value| match value {
            FieldValue::Numeric(value) => value.clone(),
            _ => 0.0
          }).unwrap_or(0.0)

      })
    }).unwrap_or(0.0);

More options and types are available under the hood and exposed through 
the documentation.

# Bugs

If you've found a bug or issue, don't hesitate to file an issue. If you are 
parsing a file, don't forget to attach it to your issue; make sure to anonymize 
the data if needed.

