# Baut die einzelne Programmdatei unter Windows. Voraussetzung: Python 3.10+ nur zum Bauen,
# die fertige .exe braucht auf dem Zielrechner nichts.
python -m pip install --upgrade pip
python -m pip install ".[raw]" pyinstaller
python -m PyInstaller --noconfirm --clean --onefile --console `
  --name fotobuch-assistent `
  --collect-all pillow_heif `
  --collect-all rawpy `
  --add-data "src/fotobuch/profiles;fotobuch/profiles" `
  --add-data "src/fotobuch/web;fotobuch/web" `
  packaging/start.py
Write-Host "Fertig: dist\fotobuch-assistent.exe"
