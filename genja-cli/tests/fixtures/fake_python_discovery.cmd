@echo off

REM Windows test double for the Python executable, not a Python module.
REM Rust tests copy this file as fake-python.cmd and run it through
REM PythonTaskDescriptorSource without requiring Python or genja-py.
if not "%~1"=="-m" goto invalid_arguments
if not "%~2"=="genja._discovery_cli" goto invalid_arguments
if not "%~3"=="--pyproject" goto invalid_arguments
if "%~4"=="" goto invalid_arguments
if not "%~5"=="" goto invalid_arguments

REM Record the command's working directory and supplied project file.
> observed_cwd.txt <nul set /p ="%CD%"
> observed_pyproject.txt <nul set /p ="%~4"

REM Optional files written by the Rust test control stderr, stdout JSON,
REM and the exit code of this simulated discovery helper.
if exist helper_stderr.txt type helper_stderr.txt 1>&2
if exist helper_stdout.txt type helper_stdout.txt
if exist helper_exit_code.txt for /f %%C in (helper_exit_code.txt) do exit /b %%C
exit /b 0

:invalid_arguments
echo unexpected discovery helper arguments 1>&2
exit /b 2
