@echo off
REM Exit on any error
setlocal enabledelayedexpansion

call ./scripts/version.bat

echo === Platform: %PLATFORM% ===
echo === Commit SHA: %COMMIT_SHA% ===

REM Set build directory
set BUILD_DIR=%cd%\build

set PACKAGE_NAME=sf_mini_core_%PLATFORM%_%VERSION%_SNAPSHOT_%COMMIT_SHA%.tar.gz
set UPLOAD_PATH=s3://sfc-eng-jenkins/universal-driver/sf_mini_core/%PACKAGE_NAME%

echo === Uploading archive to S3: %UPLOAD_PATH% ===

aws s3 cp "%BUILD_DIR%\%PACKAGE_NAME%" %UPLOAD_PATH%
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%

echo === Successfully uploaded archive to S3: %UPLOAD_PATH% ===

endlocal

