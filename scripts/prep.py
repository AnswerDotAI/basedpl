"Prepare a release: write the editor files from the glyph metadata, then export, test and clean the notebooks, and render the README."
from basedpl.editors import write
from nbdev.quarto import prepare

write()
prepare()
