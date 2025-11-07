@echo off
echo Building gcal-imp...
make prod-build

echo Installing gcal to local bin directory...
if not exist "%USERPROFILE%\bin" mkdir "%USERPROFILE%\bin"
copy "target\release\gcal-imp.exe" "%USERPROFILE%\bin\gcal.exe"

echo Installation complete! Run 'gcal' to start the application.
