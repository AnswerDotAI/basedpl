A language's '''array model''' is the definition and interpretation of its [[array]] type, which guides how data is represented. A major division between array languages is the way that nested data is handled, for example if the programmer would like to store a list of arrays with varying shapes in a single variable. The [[#Flat array theory|flat array model]] of early APLs did not allow for such structures, forcing programmers to use padding or partitioning techniques that could be difficult or unsuitable for some problems. Modern APL dialects overwhelmingly use the [[#Nested array theory|nested array model]], in which arrays contain other arrays directly, making no distinction between single numbers and characters and larger arrays. Other array languages are more varied but rarely use the nested model. [[J]] and [[Uiua]] use a historical competitor to the nested model, extending the flat model with [[#Boxes|boxes]] as introduced by [[SHARP APL]]. Other APL-influenced languages often use [[#Nested list model|nested lists]] as a simpler alternative to a built-in multi-dimensional type.

== Flat array theory ==

In [[Iverson notation]] arrays were considered to contain numbers (or [[Boolean]]s, before these were unified with ordinary numbers), which are not themselves arrays. The property that array elements are some non-array type is the defining feature of flat array theory.

Flat APLs impose the rule that all elements of arrays have the same type, such as all character or all numeric. IBM's [[APL\360]] was likely the first to specify this rule explicitly, and it has been maintained in newer languages such as [[SHARP APL]] and [[J]]. While rarely implemented, it is possible to discard this rule to obtain an inhomogeneous array theory that allows arrays to contain elements of mixed type, but not other arrays.

=== Boxes ===
{{Main|Box}}

In order to allow programmers to work with inhomogeneous or nested data, flat array languages may define a special kind of element which "encloses" or "boxes" an array. Then there are three allowed element types for an array: character, numeric, and boxed.

While a boxed array represents a collection of arrays, it is not considered to contain those arrays—its [[element]]s are boxes, and not their contents. For this reason [[scalar function]]s do not reach into boxes: they act on the elements of an array directly. Thus [[Equal to]] on two boxes compares them, with a single Boolean result indicating whether the arrays inside the boxes [[match]].

== Nested array theory ==
{{Main|Nested array model}}

A competing APL array model was developed to more transparently handle nested data, without the need to explicitly box and unbox arrays. In it, arrays contain other arrays directly. Several dialects using this "nested" model were released in the early 1980s, including [[NARS]], [[Dyalog APL]], and [[APL2]], and it's now by far the most common model used in APL dialects.

Numbers and characters in nested APLs are represented as arrays called [[simple scalar]]s. A simple scalar is a [[scalar]] with itself as an element; simple scalars are the only kind of array that can contain themselves. An array containing only simple scalar elements is called [[Simple array|simple]], and a scalar array containing a simple scalar is defined to be equivalent to that simple scalar: thus any simple, scalar array must be a simple scalar. Non-simple arrays are called [[Nested array|nested]]. The simple arrays are a superset of the arrays allowed in flat array theory without boxes: they include all arrays of numbers and characters, as well as arrays which mix numbers and characters. Arrays which would not be representable in flat array theory—those which contain a mixture of simple scalar types, or contain both simple scalars and other arrays—are called [[Mixed array|mixed]].

Because of the rule that a simple scalar is identical to its [[enclose]] (or any scalar array containing it), simple scalars are said to "float". Thus nested array theory may be called the "floating" model, while flat array theory is "grounded" in contrast.

Whether an array language is flat or nested depends only on the language's behavior from the programmer's perspective. Nearly all APLs use homogeneous flat arrays for implementation purposes, with pointers to enclose elements. However, the language's array model is determined by what is presented to the programmer and not what is stored in memory. If a programmer can freely mix element types within an array, and simple scalars float, then the language is nested!

== Based array theory ==

Based array theory discards the principle that all data should be stored in arrays, instead defining basic types such as characters and numbers independently of arrays and arrays as a collection type—possibly one of many—that can contain any data. This model does not have any widely accepted name, with the term "based system" introduced in an [[APL Quote Quad]] paper in 1981.<ref>Randall Mercer. [https://dl.acm.org/doi/abs/10.1145/586656.586663 "A based system for general arrays"]. [[APL Quote Quad]] Volume 12, Issue 2. 1981-12.</ref> However, as it is the natural model when arrays are added to an existing programming system, it is common in array libraries such as [[wikipedia:NumPy|NumPy]], [[wikipedia:ILNumerics|ILNumerics]], and [[wikipedia:Haskell (programming language)|Haskell]]'s [https://hackage.haskell.org/package/repa Repa], as well as the language [[wikipedia:Julia (programming language)|Julia]]. It has also been promoted with the mnemonic "APL+1" by Jacob Brickman,<ref>Jacob Brickman. "Behavior of APL primitives in a system with depth-1 scalars (APL+1)". [[APL-Germany]] ''APL-Journal'' 2019 1-2 ([https://apl-germany.de/wp-content/uploads/2020/06/APL_Journal_2019_12.pdf#page=5 pdf]).</ref> and is used by the APL-family language [[BQN]].

=== Mutable based arrays ===

In many languages with this array style, such as NumPy and Julia, the arrays are [[wikipedia:Immutable object|mutable]], meaning that copies of an array can be made, so that one copy reflects changes made to any copy. In contrast, APL operations that appear to modify an array, like [[indexed assignment]], will only change the particular copy of the array used, and can be said to create a new array rather than change an existing one: there is no special connection between the old and modified array. Mutable arrays make it possible for an array to contain itself, by replacing one element of an existing array with the whole array. This means that more values are possible than in an immutable based array language, and that some properties of immutable arrays, such as a finite [[depth]], do not hold.

== Nested list model ==

Some APL-derived languages choose not to define APL-style multidimensional arrays, instead providing a 1-dimensional list type. The most prominent examples are [[K]] and related languages, but several others such as [[Jelly]] and [[I]] use this model as well. When necessary, multidimensional arrays in these languages are represented as nested lists, with outer layers of nesting corresponding to leading axes. In this way the model represents a superset of the non-[[empty]] flat arrays, as it can represent any non-empty shape as well as ragged arrays and arrays with non-uniform depth. Some empty shapes require [[#Array prototypes|prototypes]] to be represented faithfully: an axis of length 0 implies a layer of empty arrays, but these arrays don't have any elements to carry further shape information so the shape must stop there if prototypes aren't supported.

The nested model may be seen as easier to use because it unifies [[rank]] and [[depth]], elevating [[leading axis theory]] from a convention to a fundamental property of the language. There is no distinction, for example, between the functions [[Pair]] and [[Laminate]], so that only one primitive covers both use cases. [[Enclose]] and [[Mix]] are no longer needed at all. A disadvantage of nested lists for implementations is that the natural representation of a "flat" multi-dimensional array is not flat, but uses pointers, which slows down many array operations if the last axis is short. For this reason K programming emphasizes structuring data to keep a long axis at the end. Pointers are also faster for operations that rearrange or select from a leading axis, as only the pointers need to be moved, leaving rows in place. In this case a nested APL programmer may prefer to [[split]] their array to get a similar advantage.

== Other features of the array model ==

=== Array prototypes ===

{{Main|Array prototype}}

An empty array may carry additional information to indicate what type its elements would be, if it had any. In flat array theory, this is typically just the type: character, numeric, or boxed. In a nested array language, the prototype may be quite complex, containing an entire nested structure.

An array's prototype is used to determine the value of [[Fill element|fills]] when they are required by the language.

=== Numeric type coercion ===

Most APLs, flat or nested, implicitly store simple numeric arrays as one of many [[numeric type]]s. When a numeric array is formed from numbers with different types, all numbers are converted to a common type in order to be represented as a flat array. If the hierarchy of numeric types is not strict, that is, there are some pairs of numeric types for which neither type is a subset of the other, then this coercion may affect the behavior of the numbers in the array. For example, [[J]] on a 64-bit machine uses both 64-bit integers and [[wikipedia:IEEE_754|double-precision floats]]. [[Catenate|Catenating]] the two results in an array of doubles, which will lose precision for integers whose absolute value is larger than 2<sup>53</sup>. In [[Dyalog APL]] a similar issue occurs with [[decimal float]]s and [[complex number]]s: combining the two results in an array of complex numbers, but this loses precision since Dyalog's complex numbers are stored as pairs of double-precision floats and its 128-bit decimal floats have higher precision than doubles.

== References ==
<references />
{{APL features}}[[Category:Arrays]]

