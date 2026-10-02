import xml.etree.ElementTree as ET
import pytest
from fastcore.test import test_eq as teq
from basedpl import bpl, BplError
from basedpl.worker import Worker


def test_svg_display_and_mimefields():
    pic = bpl('circle←•element "circle" ⋄ pic←•svg ["r":20] circle ⍬')
    root = ET.fromstring(pic._repr_mimebundle_()['image/svg+xml'])
    teq(root.tag, '{http://www.w3.org/2000/svg}svg')
    teq(root.attrib['viewBox'], '0 0 100 100')
    result = bpl('⎕←"before" ⋄ pic ⋄ ⎕←"after"', 'repl')
    teq([e['kind'] for e in result.events], ['explicit', 'display', 'explicit'])
    teq(result.events[1]['data']['image/svg+xml'], pic._repr_mimebundle_()['image/svg+xml'])
    bpl('pic.children.attrs.r←30')
    teq(ET.fromstring(bpl('pic')._repr_mimebundle_()['image/svg+xml'])[0].attrib['r'], '30')
    bpl('x←["items":[1 2 3]] ⋄ x._mime←{["text/plain":⍕+/⍵.items]}')
    teq(bpl('x')._repr_mimebundle_(), {'text/plain': '6'})
    bpl('x.items+←10')
    teq(bpl('x')._repr_mimebundle_(), {'text/plain': '36'})
    bpl('bad←["items":[1 2]] ⋄ bad._mime←{1÷"a"}')
    teq(list(bpl('bad', 'repl').events[0]['data']), ['text/plain'])
    teq(list(bpl('bad')._repr_mimebundle_()), ['text/plain'])
    with pytest.raises(BplError): bpl('•mime bad')
    teq(int(bpl('2+3')), 5)
    teq(ET.fromstring(pic._repr_mimebundle_()['image/svg+xml'])[0].attrib['r'], '20')


def test_plot_labels():
    "Keys, axis names and settings become the text a plot draws. Legends are chosen, and labels move apart."
    def texts(code):
        root = ET.fromstring(bpl(f'"image/svg+xml"⊃•mime {code}').py)
        return {e.text.strip(): float(e.get('y')) for e in root.iter('{http://www.w3.org/2000/svg}text')}
    bpl('sales←["city":["London" "Paris"] "month":["Jan" "Feb" "Mar"]]:[10 20 30 ⋄ 40 50 60]')
    plain = texts('•plot sales')
    assert {'Jan', 'Feb', 'Mar', 'month'} <= plain.keys() and 'London' not in plain
    assert {'London', 'Paris', 'city'} <= texts('["legend":"top-left"] •plot sales').keys()
    bpl('p←•plot 1 10 100 ⋄ p.y.scale←"log" ⋄ p.x.ticks←["first":0 "last":2]')
    assert {'1', '10', '100', 'first', 'last'} <= texts('p').keys()
    bpl('top←["title":"Top"] •plot sales ⋄ fig←["title":"Both"] •plot [top ⍬] ⋄ bars←["mark":"bar" "labels":1] •plot 5 7')
    assert {'Both', 'Top'} <= texts('fig').keys()
    assert {'5', '7'} <= texts('bars').keys()
    ends = texts('["legend":"end"] •plot ["x":[1 2] "aa":[3 4] "bb":[3 4]]')
    assert abs(ends['aa'] - ends['bb']) >= 10
    close = texts('["mark":"point" "labels":["aa" "bb" ""]] •plot ["x":[1 1.01 10] "v":[5 5 5]]')
    assert abs(close['aa'] - close['bb']) >= 10

def test_plot_cells():
    "Cell rows run down the plot as the matrix prints, and each run of equal colours in a row is one rectangle."
    root = ET.fromstring(bpl('"image/svg+xml"⊃•mime ["mark":"cell" "colorbar":$t] •plot ["city":["London" "Paris"] "month":["Jan" "Feb"]]:[10 20 ⋄ 30 60]').py)
    y = {e.text.strip(): float(e.get('y')) for e in root.iter('{http://www.w3.org/2000/svg}text')}
    assert y['London'] < y['Paris'] and {'10', '60'} <= y.keys()
    root = ET.fromstring(bpl('"image/svg+xml"⊃•mime "cell" •plot [1 1 2 ⋄ 3 3 3]').py)
    teq(len(root.findall('{http://www.w3.org/2000/svg}rect')), 4)

def test_svg_worker():
    with Worker() as w:
        result = w.eval('⎕←"picture" ⋄ •svg ⍬')
        assert result['error'] is None
        teq(result['value']['axis_keys'], [['tag', 'attrs', 'children']])
        teq([e['kind'] for e in result['output']], ['explicit', 'display'])
        teq(ET.fromstring(result['output'][1]['data']['image/svg+xml']).tag, '{http://www.w3.org/2000/svg}svg')
