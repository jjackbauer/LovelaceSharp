@echo off
set BIN=C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe
set D=C:\Users\ricar\dev\LovelaceSharp\docs\goal-cycle-6\round-23\r23-R
if not exist "%D%\raw" mkdir "%D%\raw"
if not exist "%D%\plots" mkdir "%D%\plots"
call :run c01 --file %D%\ls\c01.ls
call :run c02 --file %D%\ls\c02.ls
call :run c03 --file %D%\ls\c03.ls
call :run c04 --file %D%\ls\c04.ls
call :run c05 --file %D%\ls\c05.ls
call :run c06 --file %D%\ls\c06.ls
call :run c07 --file %D%\ls\c07.ls
call :run c08 --file %D%\ls\c08.ls
call :run c09 --file %D%\ls\c09.ls --plot-dir %D%\plots
call :run c10 --file %D%\ls\c10.ls --plot-dir %D%\plots
call :run c11 --file %D%\ls\c11.ls
call :run c11t --file %D%\ls\c11.ls --text
call :run c11to --file %D%\ls\c11.ls --text --omit-variables
call :run c12 --file %D%\ls\c12.ls --cancel-after 60
call :run c13 --file %D%\ls\c13.ls --cancel-after 30
call :run c14 --file %D%\ls\c14.ls
call :run c15 --file %D%\ls\c15.ls
call :run c16 --file %D%\ls\c16.ls
call :run c17 --file %D%\ls\c17.ls
call :run c18 --file %D%\ls\c18.ls --print-budget 20
call :run c19 --file %D%\ls\c19.ls
call :run c20 --file %D%\ls\c20.ls
call :run c21 --file %D%\ls\c21.ls
call :run c21crlf --file %D%\ls\c21-crlf.ls
call :run c21bom --file %D%\ls\c21-bomcrlf.ls
call :run c21nonl --file %D%\ls\c21-nonl.ls
call :run c22 --file %D%\ls\c22.ls
call :run c23 --file %D%\ls\c23.ls
call :run c24 --file %D%\ls\c24.ls
call :run c25 --file %D%\ls\c25.ls
call :run c26stdin --stdin < %D%\ls\c11.ls
call :run c27eval --eval 1+1
call :run c28missing --file %D%\ls\nosuch.ls
exit /b 0
:run
"%BIN%" %2 %3 %4 %5 %6 %7 %8 > "%D%\raw\%1.out" 2> "%D%\raw\%1.err"
echo %ERRORLEVEL% > "%D%\raw\%1.rc"
exit /b 0
