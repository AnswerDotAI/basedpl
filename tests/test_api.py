import operator
from fractions import Fraction
import numpy as np
import pytest
from miniapl import (Array, Session, AplError, plus, times, subtract, divide, power, sign, tally, iota,
                     reshape, shape, floor, logarithm, reverse, transpose, fork, atop)


def test_array_surface():
    a = Array([[1, 2, 3], [4, 5, 6]])
    assert len(a) == 2
    np.testing.assert_array_equal(a + [10, 20], [[11, 12, 13], [24, 25, 26]])
    np.testing.assert_array_equal(10 - a, [[9, 8, 7], [6, 5, 4]])
    np.testing.assert_array_equal(a[2, :], [4, 5, 6])
    np.testing.assert_array_equal(a[:, [1, 3]], [[1, 3], [4, 6]])
    np.testing.assert_array_equal(list(a)[0], [1, 2, 3])
    with pytest.raises(AplError): a[0, 1]
    with pytest.raises(TypeError): a[1:2, :]
    assert (Array(7) % 3).py == 1 and (Array(7) // 3).py == 2
    assert (Array(2) ** 3).py == 8 and (Array(3) / 2).py == Fraction(3, 2)
    assert operator.index(Array(3)) == 3 and int(Array(3.5)) == 3 and float(Array(Fraction(1, 2))) == .5
    assert bool(Array([1])) and not bool(Array(0))
    with pytest.raises(ValueError): bool(a)
    with pytest.raises(TypeError): operator.index(Array(3.))
    with pytest.raises(TypeError): hash(a)
    np.testing.assert_array_equal(a > 3, [[0, 0, 0], [1, 1, 1]])
    np.testing.assert_array_equal(~(a > 3), a <= 3)
    np.testing.assert_array_equal(a @ transpose(a), [[14, 32], [32, 77]])
    assert Array(2).apl == '2x' and Array(2.).apl == '2'
    assert repr(Array(2)) != repr(Array(2.))


def test_words_binding_and_operators():
    np.testing.assert_array_equal(iota(3), [1, 2, 3])
    np.testing.assert_array_equal(shape(reshape([2, 3], iota(6))), [2, 3])
    assert times(2.)(3).py == 6. and type(times(2.)(3).py) is float
    assert times(2)(3).py == 6 and type(times(2)(3).py) is int
    assert subtract(2.)(5).py == 3 and subtract.left(2.)(5).py == -3
    assert divide.left(1)(3).py == Fraction(1, 3)
    assert sign(-3).py == -1 and power(2)(3).py == 9
    mean = plus.reduce() / tally
    assert mean([1, 2, 4]).py == Fraction(7, 3)
    assert fork(plus.reduce(), divide, tally)([1, 2, 4]).py == Fraction(7, 3)
    assert (plus.reduce() + 2)([1, 2]).py == 5
    assert (2 - plus.reduce())([1, 2]).py == -1
    np.testing.assert_array_equal(plus.scan()([1, 2, 3]), [1, 3, 6])
    np.testing.assert_array_equal(subtract.scan()([1, 2, 3]), [1, -1, -4])
    np.testing.assert_array_equal(plus.scan()(10, [1, 2, 3]), [11, 13, 16])
    assert type(tally('abc').py) is int and shape(Array([1., 2.])).apl == '2x'
    assert (Array('abc') + 1).py == 'bcd'
    np.testing.assert_array_equal(plus.reduce()[1](Array([[1, 2], [3, 4]])), [4, 6])
    np.testing.assert_array_equal(times.outer()([1, 2], [3, 4]), [[3, 4], [6, 8]])
    assert (plus @ times)([1, 2], [3, 4]).py == 11
    assert (subtract(1) ** 3)(10).py == 7
    assert (times(2.) ** -1)(10).py == 5
    assert (floor << times(10.))(1.25).py == 12
    assert (times(10.) >> floor)(1.25).py == 12
    assert atop(floor, times(10.))(1.25).py == 12
    assert plus.under(logarithm)(2., 3.).py == pytest.approx(6.)
    np.testing.assert_array_equal(reverse.rank(1)(Array([[1, 2], [3, 4]])), [[2, 1], [4, 3]])
    np.testing.assert_array_equal(times(2).at([2, 4])([1, 2, 3, 4]), [1, 4, 3, 8])
    with pytest.raises(TypeError): plus @ Array(2)
    with pytest.raises(ValueError, match='DOMAIN'): plus.over(3)
    with pytest.raises(ValueError, match='DOMAIN'): plus.stencil(times)


def test_retained_and_late_bound_functions(capsys):
    with Session() as a, Session() as b:
        with pytest.raises(AplError) as caught: a.fn('{')
        assert caught.value.source == '{'
        a(k=2)
        f = a('{k+⍵}')
        assert f(3).py == 5
        a(k=10)
        assert f(3).py == 13
        late = a.fn('g')
        a('g←+')
        composed = late + times(2)
        assert composed(3).py == 9
        assert late.reduce()([]).py == 0
        assert (a.fn('+') @ a.fn('×'))([], []).py == 0
        np.testing.assert_array_equal(a.fn('⌽')[1]([[1, 2], [3, 4]]), [[3, 4], [1, 2]])
        a('scale←2∘×')
        assert (a.fn('scale') ** -1)(6).py == 3
        a(fold=late.reduce())
        assert a('fold ⍬').py == 0
        a(loop=a.fn('loop'))
        with pytest.raises(AplError, match='LIMIT'): a('loop 1')
        assert a('1+1').py == 2
        assert a('count←{⍵=0:0 ⋄ 1+∇⍵-1} ⋄ count 500').py == 500
        a('g←-')
        assert composed(3).py == 3
        saved = a('g')
        a('g←+')
        assert saved(3).py == -3 and late(3).py == 3
        assert b('f 5', f=times(2)).py == 10
        with pytest.raises(ValueError, match='session'): b(f=f)
        with pytest.raises(ValueError, match='session'): f + b.fn('+')
        a('outer←{k←100 ⋄ ⍺⍺ ⍵}', f=f)
        assert a('f outer 1').py == 11
        g = a('{⎕←⍵ ⋄ ⍵}')
        assert (g + g)(2).py == 4 and capsys.readouterr().out == '2x\n2x\n'
        assert a.eval('g', g=g).value(2).py == 2
        assert capsys.readouterr().out == '2x\n'
    with pytest.raises(RuntimeError, match='closed'): f(3)
    assert saved(3).py == -3
