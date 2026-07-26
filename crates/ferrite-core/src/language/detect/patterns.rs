pub const LANGUAGES: &[(&str, &[(&str, i32)])] = {
    &[
        (
            "c",
            &[
                // Primitive variable declaration.
                (r#"(char|long|int|float|double)( )+\w+( )*=?"#, 2),
                // malloc function call
                (r#"malloc\(.+\)"#, 2),
                // #include <whatever.h>
                (r#"#include (<|")\w+\.h(>|")"#, 2),
                // pointer
                (r#"(\w+)( )*\*( )*\w+"#, 2),
                // Variable declaration and/or initialisation.
                (r#"(\w+)( )+\w+(;|( )*=)"#, 1),
                // Array declaration.
                (r#"(\w+)( )+\w+\[.+\]"#, 1),
                // #define macro
                (r#"#define( )+.+"#, 1),
                // NULL constant
                (r#"NULL"#, 1),
                // void keyword
                (r#"void/g"#, 1),
                // (else )if statement
                (r#"(else )?if( )*\(.+\)"#, 1),
                // while loop
                (r#"while( )+\(.+\)"#, 1),
                // printf function
                (r#"(printf|puts)( )*\(.+\)"#, 1),
                // new Keyword from C++
                (r#"new \w+"#, -1),
                // Single quote multicharacter string
                (r#"'.{2,}'"#, -1),
                // JS variable declaration
                (r#"var( )+\w+( )*=?"#, -1),
            ],
        ),
        (
            "cpp",
            &[
                // Primitive variable declaration.
                (r#"(char|long|int|float|double)( )+\w+( )*=?"#, 2),
                // #include <whatever.h>
                (r#"#include( )*(<|")\w+(\.h)?(>|")"#, 2),
                // using namespace something
                (r#"using( )+namespace( )+.+( )*;"#, 2),
                // template declaration
                (r#"template( )*<.*>"#, 2),
                // std
                (r#"std::\w+"#, 2),
                // cout/cin/endl
                (r#"(cout|cin|endl)"#, 2),
                // Visibility specifiers
                (r#"(public|protected|private):"#, 2),
                // nullptr
                (r#"nullptr"#, 2),
                // new Keyword
                (r#"new \w+(\(.*\))?"#, 1),
                // #define macro
                (r#"#define( )+.+"#, 1),
                // template usage
                (r#"\w+<\w+>"#, 1),
                // class keyword
                (r#"class( )+\w+"#, 1),
                // void keyword
                (r#"void"#, 1),
                // (else )if statement
                (r#"(else )?if( )*\(.+\)"#, 1),
                // while loop
                (r#"while( )+\(.+\)"#, 1),
                // Scope operator
                (r#"\w*::\w+"#, 1),
                // Single quote multicharacter string
                (r#"'.{2,}'"#, -1),
                // Java List/ArrayList
                (r#"(List<\w+>|ArrayList<\w*>( )*\(.*\))(( )+[\w]+|;)"#, -1),
            ],
        ),
        (
            "diff",
            &[
                (r#"@@ "#, 5),
                (r#"^diff --git"#, 15),
                (r#"^---"#, 3),
                (r#"^\+\+\+"#, 3),
                (r#"^+"#, 1),
                (r#"^-"#, 1),
            ],
        ),
        (
            "python",
            &[
                (r#"^#!(/usr)*/bin/python\d*"#, 15),
                (r#"^#!/usr/bin/env python\d*"#, 15),
            ],
        ),
        (
            "bash",
            &[
                (r#"^#!(/usr)*/bin/bash"#, 15),
                (r#"^#!/usr/bin/env bash"#, 15),
            ],
        ),
        (
            "fish",
            &[
                (r#"^#!(/usr)*/bin/fish"#, 15),
                (r#"^#!/usr/bin/env fish"#, 15),
            ],
        ),
        ("html", &[(r#"^<!DOCTYPE html>"#, 15)]),
        ("xml", &[(r#"^<?xml"#, 15)]),
    ]
};
