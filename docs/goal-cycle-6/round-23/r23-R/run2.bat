@echo off
set BIN=C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe
set D=C:\Users\ricar\dev\LovelaceSharp\docs\goal-cycle-6\round-23\r23-R
call :run c29 --file %D%\ls\c29.ls
call :run c30 --file %D%\ls\c30.ls
call :run c31 --file %D%\ls\c31.ls
call :run c32 --file %D%\ls\c32.ls
call :run c33 --file %D%\ls\c33.ls
call :run c34 --file %D%\ls\c34.ls
call :run c35 --file %D%\ls\c35.ls
call :run u01 --file %D%\ls\u01.ls --text
call :run u02 --file %D%\ls\u02.ls --text
exit /b 0
:run
"%BIN%" %2 %3 %4 %5 %6 %7 %8 > "%D%\raw\%1.out" 2> "%D%\raw\%1.err"
echo %ERRORLEVEL% > "%D%\raw\%1.rc"
exit /b 0
