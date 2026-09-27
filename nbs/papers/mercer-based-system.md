# A Based System for General Arrays

**Randall Mercer**

*APL Quote Quad* 12 2, December 1981, pp. 18–21.

Manually transcribed from the page images in `basedpaper.pdf`. The preceding article's ending on page 18 is omitted. Enclose and Disclose are normalized from the author's IPSA spellings `<` and `>` to `⊂` and `⊃`. The symbol `↮` represents the source's crossed-out equivalence arrow.

## Abstract

The “floating” and “grounded” systems for including general arrays in APL are compared, and their deficiencies discussed. Finally, an alternative grounded system, the “based” system, is defined; in it the floating and grounded systems can be modeled easily and these deficiencies do not appear.

## Introduction

Methods for allowing arrays to contain other arrays have long been discussed in the APL community. Two fundamentally different approaches have been developed. One, frequently called the “floating system” (FS), is based on the array theory of Trenchard More and has become the basis for an experimental Nested Arrays System at STSC, Inc. [1]. The other, frequently called the “grounded system” (GS), is the basis for a recent extension [2] of the I.P. Sharp Associates production system to include “enclosed arrays”. Both systems define functions Enclose and Disclose, represented by `<` and `>` at IPSA and by `⊂` and `⊃` at STSC. [The author uses the IPSA spellings throughout the original; this transcription uses `⊂` and `⊃`.]

For convenience in the ensuing discussion, I shall consider only monadic functions and arrays of rank 0 or 1 only. In presentations below, except where further restricted, *A* represents any array, *SA* represents any simple array (as in APL without general arrays), and *S* represents a simple scalar.

In both FS and GS:

```apl
⍴⍴⊂A  ↔  0
⊃⊂A   ↔  A
⊃S    ↔  S
SA[S] yields a scalar
```

In FS:

```apl
⊂S  ↔  S
```

but in GS:

```apl
⊂S  ↮  S
```

I shall not discuss the extensions to `⊃` made by STSC and IPSA to apply to arrays of rank exceeding 0.

## What is an Element?

I must digress here to define “element” and “contains”, lest confusion reign. I define element *S* of *A* through an explicit disclose, as:

```apl
⊃A[S]
```

Alternatively, *A* contains `⊃A[S]` at its position *S*. The justification for this definition of element is that indexed selection in APL is not really an element-selection function, but a function for selecting a subarray—it extracts elements from the source of the selection and puts them inside an array whose shape is given by the shape of the index.

This definition of element is analogous to that of set theory, where one distinguishes between an element of a set and a subset containing that element. For example, consider the set Q defined by:

```text
Q={{1,2},{3,4},{5,6}}
```

It contains three elements, i.e.:

```text
{1,2}, {3,4}, and {5,6}
```

each of which is less deeply nested than Q. Each nonempty subset of Q, e.g., `{{1,2}}`, is as deeply nested as Q. The operation analogous to indexed selection in APL is selection of a subset, e.g., `{{1,2}}`, which must be disclosed to get an element of Q—in this case, `{1,2}`.

In both FS and GS, then, simple arrays contain simple scalars as their elements, and the element of `,⊂A` is *A*, not `⊂A`. With this definition of element, GS has the curious property that both simple arrays and arrays of enclosed scalars contain scalars. I think this is the reason why those who like GS prefer to think of `A[S]`, not `⊃A[S]`, as element *S* of *A*.

## Depth in GS and FS

A function, *DEPTH*, can be defined to correspond to the intuitive notion of the depth of a general array:

```apl
DEPTH SA  ↔  1
DEPTH A   ↔  1+⌈/DEPTH¨,A    ⍝ for non-simple arrays A
```

where `¨` is STSC's operator “each”.

Notice that in GS we have the identity:

```apl
DEPTH ⊂A  ↔  1+DEPTH A
```

which fails to hold in FS for simple scalar *A*. There is, of course, an identity in FS for all rank-0 arrays *A*:

```apl
DEPTH ⊂⊃A  ↔  DEPTH A
```

that fails to hold in GS for simple scalar *A*.

## A Deficiency of the Floating System

A major usage [3] of strand notation in STSC's system has been to pass more than two arguments to a defined function. To compare the ability of FS and GS to make use of some construct for achieving this end, I define the value of `(A;B;C;...)` to be the same as the value of `(⊂A),(⊂B),(⊂C),...`. This “list” notation is similar in a floating system to the strand notation defined by STSC, but works equally well in either FS or GS. (Strand notation in GS would require the adoption of heterogeneous arrays.)

The use of list or strand notation exposes a minor disadvantage of FS—you cannot define in the obvious way a function that takes a variable number of simple arrays as arguments. For example in FS, where `(1;2;3)` is equivalent to `1 2 3`, the following behavior would not be possible:

```apl
      'I2' ⎕FMT (1;2;3)
1 2 3
```

```apl
      'I2' ⎕FMT (1 2 3)
 1
 2
 3
```

In GS, there is no problem, because `(1;2;3)` is not equivalent to `1 2 3`.

It is possible in FS to pass a variable number of simple arrays as arguments by using a one-element vector instead of a list when there is only one argument to pass. For example, `(A;B;C)` would be suitable for passing three arguments, but `,⊂A` would be used to pass a single argument. This circumvents the problem with only a minor notational inconvenience. However, it still does not allow a function to treat simple arrays quite differently from arrays of simple arrays as `⎕FMT` does in either implementation, since an array of scalars is both a simple array and an array of simple arrays in FS.

## A Deficiency of the Grounded System

Consider the “with” operator `¨` of IPSA's GS. `F¨⊃` works almost like STSC's each operator used with *F*. However, if *F* is a function such that `F A` has the same depth as *A*, then `F¨⊃SA` will exceed the depth of simple array *SA*, even though the depth of `F¨⊃A` is the same as the depth of *A* when *A* is not simple. This is a consequence of combining a permissive disclose with a strict enclose, i.e., that `DEPTH ⊂⊃S` is `1+DEPTH S`. This means that although you can use `¨` to apply a defined mixed function to each element of an array of enclosed arrays, you cannot apply in the same way a defined scalar function to a simple array of scalars. For example, using Iverson's direct-definition operator [6] on the IPSA system:

```apl
      F←' '∇'+/⍳⍴⍵'
      -F¨⊃3 2 3
DOMAIN ERROR
      -F¨⊃3 2 3
      ^
```

On STSC's system, however, we get:

```apl
      -F¨3 2 3
¯6 ¯3 ¯6
```

> Transcription note: the scan includes `⍴` in both direct-definition bodies. The triangular-number results shown here imply `+/⍳⍵`, and `⊃+/⍳⍵` in the based-system example below. The printed bodies are retained.

## The Based System

Two assumptions have been implicitly accepted by the proponents of either FS or GS:

1. Indexed selection returns an array whose elements are elements of the array to which indexed selection was applied.
2. The elements of simple arrays are simple scalars.

From these one must conclude that either:

3. Scalars contain themselves because `SA[S]` is element *S* of *A* which is a scalar.

or:

**(3′)** Array elements are not analogous to set elements—in particular, element *S* of *A* is not `⊃A[S]` but `A[S]`.

The “based system” (BS) eliminates these conclusions by replacing assumption (2) with:

**(2′)** The elements of simple arrays are disclosed scalars, which are not the same as scalars.

BS may be summarized by the following identities:

```apl
⍴⍴⊂A  ↔  0
⊃⊂A   ↔  A
⊂S    ↮  S
⊃S    ↮  S
SA[S] yields a scalar
```

Let us call `⊃S` a disclosed scalar. To extend the primitive functions of APL to disclosed scalars, I posit three further identities. Let *MSF* be any monadic scalar function and *AOF* any other primitive function; then:

```apl
⊃⊃S     ↔  Domain Error
MSF ⊃S  ↔  ⊃ MSF S
AOF ⊃S  ↔  AOF S
```

In BS, there is no problem in passing a variable number of simple-array arguments to a defined function by using list notation, since a list of scalars is distinguishable from a simple vector, with the former containing scalars and the latter containing disclosed scalars.

The with operator also displays more felicitous behavior in BS. `F¨⊃SA` has the same depth as *SA*, allowing you to define your own scalar functions (mapping disclosed scalars to disclosed scalars) and to use `¨⊃` to apply them over all the elements of an array:

```apl
      F←' '∇' ⊃+/⍳⍴⍵'
      -F¨⊃3 2 3
¯6 ¯3 ¯6
```

In BS, *DEPTH* may be defined by the following properties:

```apl
DEPTH ⊃S  ↔  0
DEPTH A   ↔  1+⌈/DEPTH¨⊃A
```

with which definition both these identities hold:

```apl
DEPTH ⊂A   ↔  1+DEPTH A
DEPTH ⊂⊃A  ↔  DEPTH A    ⍝ for A of rank 0
```

Another interesting depth-related identity holds in BS but does not hold in either FS or GS:

```apl
DEPTH A[S]  ↔  DEPTH A
```

for any array whose elements all have the same depth.

## Rephrasing the Definition of the Based System

Let us rephrase the definition of the based system, calling “scalars” what I previously called “disclosed scalars” and calling “enclosed scalars” what have previously been called “scalars”:

```apl
⍴⍴⊂A  ↔  0
⊃⊂A   ↔  A
⊂S    ↮  S
⊃S    ↔  Domain Error
SA[S] yields an enclosed scalar
```

A monadic scalar function is then a function that operates on a scalar to produce a scalar result and operates on an array of scalars to return an array of scalars in the familiar fashion.

The definitions of other functions must be modified to refer to enclosed scalars rather than to scalars, and a rule must be adopted:

> **Global Equivalence Rule for Scalars**—Except where explicitly defined otherwise, a scalar argument to a primitive function is treated as though it were an enclosed scalar.

It should be noted that some implementations already have a rule for empty arrays, though it is not always stated as such:

> **Global Equivalence Rule for Empty Arrays**—Except where explicitly defined otherwise, empty character arguments and empty numeric arguments will be treated in the same way.

Also, by extending the equivalence rule for scalars, you can get a version of the based system that behaves almost exactly like the floating system, but you can pass variable numbers of simple-array arguments without difficulty, and the identities for the depth function hold:

> **Extended Global Equivalence Rule for Scalars**—Except where explicitly defined otherwise, a scalar argument or a multiply enclosed scalar argument to a primitive function is treated as though it were an enclosed scalar.

## Depth and Rank of Functions

Iverson has suggested [5] that argument and result ranks might usefully be defined for primitive functions, where the argument rank is the rank of arrays to which a primitive function “naturally” applies and the result rank is the rank of the result after applying a primitive function to an argument of its argument rank. For example, reversal (`⌽`) has argument rank 1 and result rank 1, and matrix inversion (`⌹`) has argument rank 2 and result rank 2.

It is also possible to define argument and result “depths” for functions. In BS, scalar functions have arguments of depth 0 and results of depth 0. Disclose has a result depth of one less than the argument depth. Enclose has a result depth of one more than the argument depth. Other mixed functions have argument and result depth of 1. The function *DEPTH*, defined above, has result depth 1, but accepts an argument of any depth. `-¨⊃` is a function with result depth the same as argument depth (unlike GS, where the result depth and argument depth sometimes differ).

## Depth and FS

The difficulty with passing a variable number of simple-array arguments in FS can now be seen as a deficiency of FS in dealing with functions, such as *DEPTH* and `⎕FMT`, of arguments of arbitrary depth. I could contrive other examples, e.g., a function that sums the leaves of its argument weighted by the depths of the leaves. However, whether there are many functions of practical utility depending strongly on that feature is an open question.

## Conclusions

The floating system is inconvenient for passing a variable number of simple arrays as arguments to a defined function by means of strand or list notation. Also, the *DEPTH* function fails to satisfy a simple identity that is satisfied in both the grounded and based systems. It is also possible that there are useful functions where depth matters which the floating system does not handle well.

In the grounded system, a user cannot define a scalar function and then use the with operator and disclose to get the usual kind of extension of a scalar function to simple arrays of scalars. Some identities of the floating and based systems that involve the *DEPTH* function fail in the grounded system. These negative aspects of the grounded system are consequences of the requirement that the disclose of a simple scalar be a scalar.

The based system that I have presented does not have these deficiencies, but requires disclosed scalars to be the fundamental entities in APL, and that a rule be introduced to extend the definitions of current nonscalar primitive functions to apply to disclosed scalars as well as to scalars.

Randall Mercer  
Data General, E111  
4400 Computer Drive  
Westboro, Massachusetts  
USA 01580

## References

1. Carl M. Cheney. *APL\*PLUS Nested Arrays, Reference Manual*, STSC, Inc. (1981).
2. K.E. Iverson and R. Bernecky. Operators and enclosed arrays, *APL Users Meeting Proceedings*, I.P. Sharp Assocs. (1980) pp. 319–381.
3. Bob Smith. On strand notation (Letter to the editor in reply to [4]), *APL Quote Quad* 11 3 (March 1981) pp. 4–6.
4. K.E. Iverson. On strand notation (Letter to the editor), *APL Quote Quad* 11 3 (March 1981) pp. 3–4.
5. K.E. Iverson. Operators, *ACM Trans. on Prog. Lang. and Systems* (Oct. 1979) pp. 161–179.
6. K.E. Iverson and P.K. Wooster. A function-definition operator, *APL 81 Conference Proceedings*, *APL Quote Quad* 12 1 (Sept. 1981) pp. 142–145.
