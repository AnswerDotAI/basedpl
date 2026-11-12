from fractions import Fraction
import pytest, basedpl as b
import numpy as np
from fastcore.test import test_eq as teq


def test_expression_parts():
    fn, plus, divide = b.fn, b.plus, b.divide
    assert fn(1, plus, 1).bpl == b.bpl('1ₓ+1ₓ').bpl
    assert fn(1, divide, 2).py == 0 and fn(1., divide, 2.).py == 0.5
    assert fn((1., plus, 1.), divide, 4.).py == 0.5
    assert fn(Fraction(1, 3), plus, Fraction(1, 6)).py == Fraction(1, 2)
    assert fn('a"b', b.catenate, 'c').py == 'a"bc'
    assert fn(b.Array('a')).py == 'a'
    assert fn([[1, 2], [3, 4]]).shape == (2,)
    assert fn([1, 2, b.diamond, 3, 4]).shape == (2, 2)
    teq(fn([(1, plus, 2), b.otherwise, (3, plus, 4)]).py, [3, 7])
    teq(fn([b.plus, b.times], b.space, 2.).py, [2., 1.])
    mean = fn(plus, b.reduce, divide, b.tally)
    assert mean([2., 4., 9.]).py == 5.
    assert fn(mean, [2., 4., 9.]).py == 5.
    assert fn(plus.reduce, divide, b.tally, b.space, [2., 4., 9.]).py == 5.
    assert fn(plus.reduce, divide, b.tally, [2., 4., 9.]).py == 0
    stage = b.bpl('∘')
    matrix = b.Array(np.array([[2., 4.], [6., 8.]]))
    teq(fn(plus.reduce, divide, b.tally, stage, b.rank, 1, matrix).py, [3., 7.])
    table = b.Array(a=[1, 2, 3], b=[4, 5, 6])
    selected = fn(table, b.dot, [[1, 0]])
    assert selected.keys == (('b', 'a'),)
    assert b.match(selected, b.Array(b=[4, 5, 6], a=[1, 2, 3]))
    teq(fn(b.dot, [[0, 2]])([1, 2, 3]).py, [1, 3])
    assert b.match(fn(table, b.dot, b.each, [[0, 2]]), b.Array(a=[1, 3], b=[4, 6]))
    assert fn(b.Array([[1, 2], [3, 4]]), b.dot, [1, 0]).py == 3
    op = b.bpl('{⍶ω}')
    assert fn(plus, op)(3).py == 3
    from basedpl import ι, ρ
    teq(ι(4).py, [0, 1, 2, 3])
    teq(ρ([2, 2], ι(4)).py, [[0, 1], [2, 3]])


def test_expression_weak_numbers():
    previous = b.prefs([])['float']
    try:
        b.prefs(dict(float=32))
        narrow = b.Array(np.array([1., 2.], dtype=np.float16))
        assert b.fn(narrow, b.plus, 1.).np.dtype == b.plus(narrow, 1.).np.dtype
    finally: b.prefs(dict(float=previous))


def test_prepared_dfns_and_scripts(capsys):
    from basedpl import α, ω
    fn, a, w = b.fn, α, ω
    assert α is b.alpha and ω is b.omega
    err = fn((a-w)/a)
    stable_log = fn(b.natural_log, b.Array(1.), b.plus, w)
    assert stable_log(1e-17).py == b.fn('{⊛1+ω}')(1e-17).py
    rms = b.plus.reduce.divide.tally.under(b.square)
    assert fn(rms, err)([1., 2.], [2., 4.]).py == 1.
    inc = fn(w, b.plus, 1)
    assert fn(inc, b.plus, inc)(3).py == 8
    assert fn(b.plus-b.times)(2, 3).py == -1
    assert fn((2+w)*3)(2).py == 12
    assert fn((a-w)//2)(7, 2).py == 2
    assert fn(abs(-w))(3).py == 3
    assert fn((a<w) & (w<=3))(2, 3).py
    assert fn(w | 2)(1).py == 3
    teq(fn(a@w)([[1, 2], [3, 4]], [[5, 6], [7, 8]]).py, [[19, 22], [43, 50]])
    with pytest.raises(TypeError, match='fn'): bool(a<w)
    late = fn('expression_test_g')
    b.bpl('expression_test_g←+')
    current = fn(late, w)
    assert current(3).py == 3
    b.bpl('expression_test_g←-')
    assert current(3).py == -3
    sign = fn((w, b.less, 0), b.guard, -1, b.otherwise, (w, b.greater, 0), b.guard, 1, b.otherwise, 0)
    teq(sign.each([-3, 0, 2]).py, [-1, 0, 1])
    assert fn(w, b.plus, 1, b.diamond, w, b.times, 2)(3).py == 6
    assert fn(w, b.sup(2))(3).py == 9
    assert fn(w, b.sup(-1))(2.).py == 0.5
    matrix = b.Array([[1, 2], [3, 4]])
    teq(fn(b.plus.reduce, b.sup(1), matrix).py, [3, 7])
    assert b.match(fn(b.Array([10, 20]), b.plus, b.sup(0, 1), matrix), b.Array([[11, 12], [23, 24]]))
    assert fn(b.square, b.inverse)(9.).py == 3.
    assert fn(w, b.subs(1, 0))(matrix).py == 3
    teq(fn(b.plus.reduce, b.subs(0), matrix).py, [4, 6])
    with pytest.raises(AttributeError, match='subscript.*subtract|subtract.*subscript'): b.sub
    assert b.subs is b.subscript and b.sup is b.superscript
    with pytest.raises(b.BplError): fn([1, b.otherwise])
    assert capsys.readouterr().out == ''
