set runtimepath^=editors/vim
filetype on
syntax on
edit apl-syntax-test.apl
call setline(1, ["⍝ comment + 'text'", "'can''t ⍝ +' ⋄ ⍳3", '+/1 2', "'unfinished", '⍪⍛⌺⍠\\⇄⌾↕ℙ𝒬𝒫∂'])
call assert_equal('apl', &filetype)
for [lnum, pattern, group] in [
      \ [1, '+', 'aplComment'], [1, 'text', 'aplComment'], [2, '⍝', 'aplString'], [2, '+', 'aplString'],
      \ [2, '⍳', 'aplGlyph'], [3, '/', 'aplGlyph'], [5, '⍛', 'aplGlyph'], [5, '\\', 'aplGlyph'], [5, '𝒫', 'aplGlyph']]
  call assert_equal(group, synIDattr(synID(lnum, match(getline(lnum), pattern)+1, 1), 'name'), pattern)
endfor
if !empty(v:errors)
  echoerr join(v:errors, "\n")
  cquit
endif
qa!
