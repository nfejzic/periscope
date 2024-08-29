nadir_thesis.pdf: thesis.tex
	latexmk -jobname=nadir_thesis -pdf thesis.tex
	notify-send "Finished building thesis"
	make clean

clean:
	latexmk -jobname=nadir_thesis -c thesis.tex
