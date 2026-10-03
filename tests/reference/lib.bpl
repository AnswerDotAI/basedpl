⍝⍝ Arrays

⍝ — disp with a centring left argument centres each item in its cell
[disp]←•load "lib/array.bpl" ⋄ X←["a" "ccc" ⋄ "bb" "d"] ⋄ [0 0 disp X;0 1 disp X]
[["┌──┬───┐" ⋄ "│a │ccc│" ⋄ "├──┼───┤" ⋄ "│bb│d  │" ⋄ "└──┴───┘"] ["┌──┬───┐" ⋄ "│a │ccc│" ⋄ "├──┼───┤" ⋄ "│bb│ d │" ⋄ "└──┴───┘"]]

⍝ — disp and dsp draw each plane of a nested rank-3 array, and disp leaves a blank row between rank-4 hyperplanes
[disp dsp]←•load "lib/array.bpl" ⋄ v←'a' "bb" 'c' "dd" ⋄ [disp 2 1 2⍴v;dsp 2 1 2⍴v;disp 2 2 1 1⍴v]
[["┌─┬──┐" ⋄ "│a│bb│" ⋄ "└─┴──┘" ⋄ "┌─┬──┐" ⋄ "│c│dd│" ⋄ "└─┴──┘"] ["────" ⋄ "a│bb" ⋄ "    " ⋄ "c│dd"] ["┌──┐" ⋄ "│a │" ⋄ "└──┘" ⋄ "┌──┐" ⋄ "│bb│" ⋄ "└──┘" ⋄ "    " ⋄ "┌──┐" ⋄ "│c │" ⋄ "└──┘" ⋄ "┌──┐" ⋄ "│dd│" ⋄ "└──┘"]]

⍝⍝ Blank removal

⍝ — On a matrix, blank removal acts on columns that are blank in every row
[dab deb dlb dmb dtb dxb]←•load "lib/string.bpl" ⋄ M←["  a  b  " ⋄ " c   d  "] ⋄ [dlb M;dtb M;deb M;dmb M;dxb M;dab M]
[[" a  b  " ⋄ "c   d  "] ["  a  b" ⋄ " c   d"] [" a  b" ⋄ "c   d"] ["  a b " ⋄ " c  d "] [" a b" ⋄ "c  d"] ["ab" ⋄ "cd"]]

⍝ — A higher-rank array is trimmed as the matrix of its rows, keeping its leading shape
[dab deb dlb dmb dtb dxb]←•load "lib/string.bpl" ⋄ A←[["  ab  " ⋄ " c    "] ⋄ ["   d  " ⋄ "  e   "]] ⋄ [dlb A;dtb A;deb A;dmb A;dxb A;dab A]
[[[" ab  " ⋄ "c    "] ⋄ ["  d  " ⋄ " e   "]] [["  ab" ⋄ " c  "] ⋄ ["   d" ⋄ "  e "]] [[" ab" ⋄ "c  "] ⋄ ["  d" ⋄ " e "]] [["  ab " ⋄ " c   "] ⋄ ["   d " ⋄ "  e  "]] [[" ab" ⋄ "c  "] ⋄ ["  d" ⋄ " e "]] [["ab" ⋄ "c "] ⋄ ["d " ⋄ "e "]]]

⍝ — A nested argument is trimmed item by item
[dab deb dlb dmb dtb dxb]←•load "lib/string.bpl" ⋄ N←"  ab  " " c  d " ⋄ [dlb N;dtb N;deb N;dmb N;dxb N;dab N]
[["ab  " "c  d "] ["  ab" " c  d"] ["ab" "c  d"] [" ab " " c d "] ["ab" "c d"] ["ab" "cd"]]

⍝ — dab removes blanks from each item of an unevenly nested argument
[dab]←•load "lib/string.bpl" ⋄ dab ["a b" ["c d" "e f"]]   ⍝ ["ab" ["cd" "ef"]]

⍝⍝ Numbers

⍝ — adic decodes bijective base-26 names, as spreadsheet columns count: A is 1, Z is 26 and AA is 27
[adic]←•load "lib/numeric.bpl" ⋄ [•a adic "A";•a adic "Z";•a adic "AA";•a adic "AZ"]
1 26 27 52

⍝ — dec reads an empty string as 0 and ignores trailing blanks
[dec]←•load "lib/numeric.bpl" ⋄ dec "" "ff "   ⍝ 0 255

⍝ — The determinant of a 0×0 matrix is 1
[alt]←•load "lib/numeric.bpl" ⋄ -alt× 0 0⍴0   ⍝ 1

⍝ — phinary reads base-φ strings back as numbers, including negative ones
[phinary]←•load "lib/numeric.bpl" ⋄ phinary "¯10.01" "100.01"   ⍝ ¯2 3

⍝ — realroots gives no root for a negative discriminant, and one for a zero discriminant
[realroots]←•load "lib/numeric.bpl" ⋄ [realroots 1 0 1;realroots 1 2 1;realroots 1 ¯3 2]
[⍬ ¯1 [2 1]]

⍝⍝ Graphs

⍝ — path finds no path between vertices in different components
[path]←•load "lib/graph.bpl" ⋄ g←[1;0;3;2] ⋄ g path 0 3   ⍝ ⍬

⍝ — A minimum spanning tree of a disconnected graph leaves the unreachable vertices at ¯1
[wmst]←•load "lib/graph.bpl" ⋄ aa←⊃[[[1] [0] [3] [2]] [[5] [5] [7] [7]]] ⋄ aa wmst 0
¯1 0 ¯1 ¯1

⍝⍝ Trees

⍝ — Deleting most keys of a red-black tree keeps it valid and its keys in order
[foldl]←•load "lib/array.bpl" ⋄ [redblack]←•load "lib/tree.bpl" ⋄ put←'∪' redblack ⋄ rem←'~' redblack ⋄ chk←'?' redblack ⋄ vec←'∊' redblack
t←0⍃(put foldl) 64ₓ|7ₓ×⍳64ₓ ⋄ u←t⍃(rem foldl) 48↑64ₓ|13ₓ×⍳64ₓ ⋄ [↑chk u;vec u]
⍝ =>
[1ₓ [10 11 12 23 24 25 36 37 38 48 49 50 51 61 62 63]ₓ]

⍝ — A simple binary search tree reads a missing key as '?', is unchanged by removing one, and can remove a leaf
[foldl]←•load "lib/array.bpl" ⋄ [sbst]←•load "lib/tree.bpl" ⋄ put←'∪' sbst ⋄ get←'⍎' sbst ⋄ rem←'~' sbst ⋄ vec←'∊' sbst ⋄ t←0⍃(put foldl) [["b" 2] ["a" 1] ["c" 3]]
["z" get t;t ≡ t rem "z";vec t rem "c"]
⍝ =>
['?' $t [["a" 1] ["b" 2]]]

⍝ — A red-black tree replaces the value of an existing key, and is unchanged by removing a missing one
[foldl]←•load "lib/array.bpl" ⋄ [redblack]←•load "lib/tree.bpl" ⋄ put←'∪' redblack ⋄ rem←'~' redblack ⋄ vec←'∊' redblack ⋄ t←0⍃(put foldl) [["a" 1] ["b" 2] ["c" 3]]
[vec t put "b" 20;t ≡ t rem "z"]
⍝ =>
[[["a" 1] ["b" 20] ["c" 3]] $t]

⍝ — A splay tree replaces the value of an existing key, and is unchanged by removing a missing one
[foldl]←•load "lib/array.bpl" ⋄ [splay]←•load "lib/tree.bpl" ⋄ put←'∪' splay ⋄ rem←'~' splay ⋄ vec←'∊' splay ⋄ t←0⍃(put foldl) [["a" 1] ["b" 2] ["c" 3]]
[vec t put "b" 20;t ≡ t rem "z"]
⍝ =>
[[["a" 1] ["b" 20] ["c" 3]] $t]

⍝⍝ Dyalog library

⍝ — ary writes zero, terminating fractions and exact powers of the base, and repeats a recurring unit twice
[ary]←•load "lib/dyalog.bpl" ⋄ [2 ary 0;2 ary 0.75;2 ary ¯0.75;2 ary 8;16 ary 255;10 ary ÷7]
"0" "0.11" "¯0.11" "1000" "FF" "0.142857142857..."

⍝ — base_64 encodes and decodes the RFC 4648 test vectors
[base_64]←•load "lib/dyalog.bpl" ⋄ v←"f" "fo" "foo" "foob" "fooba" "foobar" ⋄ e←base_64¨•ucs¨v ⋄ [e;v≡•ucs¨base_64¨e]
[["Zg==" "Zm8=" "Zm9v" "Zm9vYg==" "Zm9vYmE=" "Zm9vYmFy"] $t]

⍝ — box frames text in ASCII when its third option is 1, and a simple left argument lists row dividers
[box]←•load "lib/dyalog.bpl" ⋄ M←["abc" ⋄ "def"] ⋄ [box M;[⍬ ⍬ 1] box M;[1] box M]
[["┌───┐" ⋄ "│abc│" ⋄ "│def│" ⋄ "└───┘"] ["+---+" ⋄ "|abc|" ⋄ "|def|" ⋄ "+---+"] ["┌───┐" ⋄ "│abc│" ⋄ "├───┤" ⋄ "│def│" ⋄ "└───┘"]]

⍝ — box accepts a unit border position: [1 ⍬] draws a row divider and [⍬ 1] a column divider
[box]←•load "lib/dyalog.bpl" ⋄ M←["abc" ⋄ "def"] ⋄ [[1 ⍬] box M;[⍬ 1] box M]
[["┌───┐" ⋄ "│abc│" ⋄ "├───┤" ⋄ "│def│" ⋄ "└───┘"] ["┌─┬──┐" ⋄ "│a│bc│" ⋄ "│d│ef│" ⋄ "└─┴──┘"]]

⍝ — case applies the function for the first true selector, or returns its argument when none is true
[case]←•load "lib/dyalog.bpl" ⋄ [1 0 0 -case÷case⌊ 2.5;0 1 0 -case÷case⌊ 2.5;0 0 1 -case÷case⌊ 2.5;0 0 0 -case÷case⌊ 2.5;1 1 0 -case÷case⌊ 2.5]
¯2.5 0.4 2ₓ 2.5 ¯2.5

⍝ — cmat finds no ⍺-combinations of fewer than ⍺ items
[cmat]←•load "lib/dyalog.bpl" ⋄ 3 cmat 2   ⍝ 0 3⍴0

⍝ — Cut follows J: 1 and 2 cut at copies of the first or last item, ¯1 drops the frets, and 0 reverses
[Cut]←•load "lib/dyalog.bpl" ⋄ [(+/Cut 1)1 2 1 3 4;(+/Cut ¯1)1 2 1 3 4;(+/Cut 2)5 1 4 2 3 4;(⊢Cut 0)1 2 3]
[3 8;2 7;10 9;3 2 1]

⍝ — Cut partitions both axes of a matrix, given a fret vector for each
[Cut]←•load "lib/dyalog.bpl" ⋄ [[1 0 1;1 0](⊂Cut 1)3 2⍴⍳6;[1 0 1;0 1](⊂Cut 2)3 2⍴⍳6]
[2 1⍴[[0 1 ⋄ 2 3] [4 5 ⋄]];2 1⍴[[0 1 ⋄] [2 3 ⋄ 4 5]]]

⍝ — days counts each row of a matrix of dates: 1900-01-01 is day 1, and 1900 is not a leap year
[days]←•load "lib/dyalog.bpl" ⋄ days [2000 1 1 ⋄ 1900 1 1]   ⍝ 36525 1

⍝ — dice names every kind of throw
[dice]←•load "lib/dyalog.bpl" ⋄ dice¨[6 6;1 1;3 3;3 4;2 3]
"Box Cars" "Snake Eyes" "Pair" "Seven" "Unlucky"

⍝ — iotag of a blank is empty
[iotag]←•load "lib/dyalog.bpl" ⋄ iotag ' '   ⍝ ""

⍝ — kt finds no knight's tour of a 3×3 board, whose centre no knight reaches
[kt]←•load "lib/dyalog.bpl" ⋄ ≢kt 3 3   ⍝ 0ₓ

⍝ — lisp reads 'x as (quote x), evaluates () to itself, takes the first true cond clause, and reports an unfinished list
[lisp]←•load "lib/dyalog.bpl" ⋄ [lisp "'(a b)";lisp "()";lisp "(cond ((= 2 2) 10) (else 20))";lisp "(cond ((= 1 2) 10))";lisp "(+ 1"]
[["a" "b"] ⍬ 10 ⍬ "unexpected eof"]

⍝ — lsys accepts a single rule without its list: the Koch rule applied to F and to F+F
[lsys]←•load "lib/dyalog.bpl" ⋄ [['F' "F+F-F-F+F"] lsys "F";['F' "F+F-F-F+F"] lsys "F+F"]
"F+F-F-F+F" "F+F-F-F+F+F+F-F-F+F"

⍝ — mayan draws zero as a shell, each five as a bar and each one as a dot: 1445225 is 9 0 13 1 5 in base 20
[mayan]←•load "lib/dyalog.bpl" ⋄ 0 mayan 1445225
5 1⍴[["     " ⋄ "⍟⍟⍟⍟ " ⋄ "⌹⌹⌹⌹⌹"] ["     " ⋄ " _@/ "] ["     " ⋄ " ⍟⍟⍟ " ⋄ "⌹⌹⌹⌹⌹" ⋄ "⌹⌹⌹⌹⌹"] ["     " ⋄ "  ⍟  "] ["     " ⋄ "⌹⌹⌹⌹⌹"]]

⍝ — morse decodes Morse, where / separates words
[morse]←•load "lib/dyalog.bpl" ⋄ [morse "..." "---" "...";morse "...." ".." "/" "..."]
"SOS" "HI S"

⍝ — pack_4 packs a uniform matrix as one leaf, and expands it again
[pack_4]←•load "lib/dyalog.bpl" ⋄ M←4 4⍴7 ⋄ [pack_4 M;M ≡ 0 pack_4 pack_4 M]
[[[7] [4 4]ₓ [0]ₓ] $t]

⍝ — packT expands what it packs: a run longer than 256, text holding its escape candidates, and text with nothing to pack
[packT]←•load "lib/dyalog.bpl" ⋄ {⍵ ≡ 0 packT packT ⍵}¨["hello";300⍴"ab",300⍴"a";(•ucs 0 8 10),"xyz"]
$t $t $t

⍝ — quzzle reports that a board with no space has no solutions
[quzzle]←•load "lib/dyalog.bpl" ⋄ quzzle ["ab" ⋄ "cd"]
"There are no solutions"

⍝ — ratsum normalises a left repeating unit other than 0 or 9: ...333 is ¯1/3
[ratsum]←•load "lib/dyalog.bpl" ⋄ rs←"0123456789"ratsum ⋄ [rs "<3|3|0>";"<3|3|0>" rs "<0|1|0>"]
"<0|0|3>" "<0|0|6>"

⍝ — In balanced ternary, with digits T 0 1, 1+1 is 1T and the negative of 1 is T
[ratsum]←•load "lib/dyalog.bpl" ⋄ bt←"T01"ratsum ⋄ ["<0|1|0>" bt "<0|1|0>";bt "<0|1|0>"]
"<0|1T|0>" "<0|T|0>"

⍝ — sudoku solves the puzzle from Wikipedia's Sudoku article
[sudoku]←•load "lib/dyalog.bpl" ⋄ sudoku [
5 3 0 0 7 0 0 0 0 ⋄
6 0 0 1 9 5 0 0 0 ⋄
0 9 8 0 0 0 0 6 0 ⋄
8 0 0 0 6 0 0 0 3 ⋄
4 0 0 8 0 3 0 0 1 ⋄
7 0 0 0 2 0 0 0 6 ⋄
0 6 0 0 0 0 2 8 0 ⋄
0 0 0 4 1 9 0 0 5 ⋄
0 0 0 0 8 0 0 7 9
]
⍝ =>
[[
5 3 4 6 7 8 9 1 2 ⋄
6 7 2 1 9 5 3 4 8 ⋄
1 9 8 3 4 2 5 6 7 ⋄
8 5 9 7 6 1 4 2 3 ⋄
4 2 6 8 5 3 7 9 1 ⋄
7 1 3 9 2 4 8 5 6 ⋄
9 6 1 5 3 7 2 8 4 ⋄
2 8 7 4 1 9 6 3 5 ⋄
3 4 5 2 8 6 1 7 9
]]

⍝ — sudoku guesses and backtracks to solve Arto Inkala's 2010 puzzle, whose solution Peter Norvig's sudoku essay publishes
[sudoku]←•load "lib/dyalog.bpl" ⋄ sudoku [
0 0 5 3 0 0 0 0 0 ⋄
8 0 0 0 0 0 0 2 0 ⋄
0 7 0 0 1 0 5 0 0 ⋄
4 0 0 0 0 5 3 0 0 ⋄
0 1 0 0 7 0 0 0 6 ⋄
0 0 3 2 0 0 0 8 0 ⋄
0 6 0 5 0 0 0 0 9 ⋄
0 0 4 0 0 0 0 3 0 ⋄
0 0 0 0 0 9 7 0 0
]
⍝ =>
[[
1 4 5 3 2 7 6 9 8 ⋄
8 3 9 6 5 4 1 2 7 ⋄
6 7 2 9 1 8 5 4 3 ⋄
4 9 6 1 8 5 3 7 2 ⋄
2 1 8 4 7 3 9 5 6 ⋄
7 5 3 2 9 6 4 8 1 ⋄
3 6 7 5 4 2 8 1 9 ⋄
9 8 4 7 6 1 2 3 5 ⋄
5 2 1 8 3 9 7 6 4
]]

⍝ — sudoku finds no solution when a cell's row, column and box already hold every digit
[sudoku]←•load "lib/dyalog.bpl" ⋄ sudoku [0 1 0 0 ⋄ 0 4 0 0 ⋄ 2 0 0 0 ⋄ 3 0 0 0]   ⍝ ⍬

⍝ — tokens reads a system name, and a doubled ⍺⍺ or ∇∇, as single tokens
[tokens]←•load "lib/dyalog.bpl" ⋄ [tokens "⎕IO←0";tokens "⍺⍺ ∇∇"]
["⎕IO" "←" "0";"⍺⍺" " " "∇∇"]

⍝ — von capitalises the letter after each listed prefix, given several prefixes, one prefix, or prefixes and exceptions
[von]←•load "lib/dyalog.bpl" ⋄ ["mc" "mac" von "angus mcdonald and ian macbeth";"mc" von "angus mcdonald";[["mc" "mac"] ""] von "ian macbeth"]
"Angus McDonald and Ian MacBeth" "Angus McDonald" "Ian MacBeth"

⍝ — words with a plain alphabet splits a string into its words and the text between them
[words]←•load "lib/dyalog.bpl" ⋄ "abc" words "ab1c ba"
"ab" "1" "c" " " "ba"

⍝ — dots marks the indentation under each closing brace of a nested dfn, with a chosen dot
[dots]←•load "lib/dyalog.bpl" ⋄ '.' dots ⊃"f←{" "    g←{" "        ⍵+1" "    }" "    g ⍵" "}"
["f←{        " ⋄ ".   g←{    " ⋄ ".   .   ⍵+1" ⋄ ".   }      " ⋄ ".   g ⍵    " ⋄ "}          "]

⍝ — dots marks the indentation above :Else and :EndIf
[dots]←•load "lib/dyalog.bpl" ⋄ dots ⊃" r←f x" " :If x" "     r←1" " :Else" "     r←2" " :EndIf"
[" r←f x  " ⋄ " :If x  " ⋄ " ·   r←1" ⋄ " :Else  " ⋄ " ·   r←2" ⋄ " :EndIf "]

⍝ — mac: a backslash stops the next character expanding, and / repeats a character by the count before the macro name
[mac]←•load "lib/dyalog.bpl" ⋄ [mac "?=x  ?? \??\? ??";mac " ∆=/+ <1∆> <2∆> <3 ∆> <4∆> <12∆>";mac "l=[-/<+/>] 2l 1l 0l"]
" xx ?x? xx" " <+> <++> <3 > <++++> <++++++++++++>" "[-<<+>>] [-<+>] [-+]"

⍝ — mac: a later definition replaces an earlier one, a block keeps its definitions local, and a body expands names when defined unless escaped
[mac]←•load "lib/dyalog.bpl" ⋄ mac¨"a=KO a=OK a" "a=K (a=O a)a" "a=OK b=a a=KO b" "a=KO b=\a a=OK b" "O=? b=\\\OK b" "a=K b=(a=O a) ba"
"OK" "OK" "OK" "OK" "OK" "OK"

⍝ — baby runs a program that uses every instruction: both jumps, both compare outcomes, both subtract codes, load, store and halt
[baby]←•load "lib/dyalog.bpl" ⋄ m←⌽(32⍴2)⊤32↑0 16404 49152 57344 32789 24598 8215 57344 57344 16406 49152 40981 24600 25 57344 57344 0 0 0 0 3 1 0 2 0 14 ⋄ 2⊥⌽baby(m)
0 16404 49152 57344 32789 24598 8215 57344 57344 16406 49152 40981 24600 25 57344 57344 0 0 0 0 3 1 4294967292 2 3 14 0 0 0 0 0 0

⍝ — ary rounds a fraction whose digits run past its limit, and marks the result with ?
[ary]←•load "lib/dyalog.bpl" ⋄ [10 ary π 1;10 ary 2*÷2;16 ary π 1]
"3.1415926535898?" "1.4142135623731?" "3.243F6A8885A?"

⍝ — Cut 3 tiles a matrix by the given movements and sizes, keeping partial tiles; ¯3 keeps only whole tiles, and a negative size reverses each tile
[Cut]←•load "lib/dyalog.bpl" ⋄ x←5 7⍴1+⍳35 ⋄ [[2 1 ⋄ 3 2]({+/,⍵}Cut 3)x;¯3 2({↑,⍵}Cut ¯3)x]
[[51 57 63 69 75 81 42 ⋄ 135 141 147 153 159 165 84 ⋄ 59 61 63 65 67 69 35] [15 16 17 18 19 20 ⋄ 22 23 24 25 26 27 ⋄ 29 30 31 32 33 34]]

⍝ — lisp reports a missing end of input at the top level, inside a list and after a quote, and literals of 20 characters; - and * apply to their arguments
[lisp]←•load "lib/dyalog.bpl" ⋄ [lisp "";lisp "  ";lisp 20⍴"1";lisp 20⍴"a";lisp "'";lisp "(a '";lisp "(a 'b";lisp "(- 10 (* 2 3))"]
"unexpected eof" "unexpected eof" "numeric literal too long" "atom too long" "unexpected eof" "unexpected eof" "unexpected eof" 4
