; source https://github.com/helix-editor/helix

[
  (true)
  (false)
] @constant.builtin.boolean
(null) @constant.builtin
(number) @constant.numeric
(pair
  key: (_) @key)

(string) @string
(escape_sequence) @constant.character.escape
(ERROR) @error

"," @punctuation.delimiter
[
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket
