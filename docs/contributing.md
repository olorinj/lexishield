# Guía de Contribución y Compilación

Este documento explica cómo configurar el entorno de desarrollo y compilar LexiShield para múltiples plataformas.

## 🛠️ Desarrollo Remoto y Compilación Cruzada Multiplataforma

Dada la exigencia de generar binarios nativos sin alertar a los EDR (antivirus) y la necesidad de compilación cruzada hacia Windows y macOS desde entornos aislados, **el único método oficial de compilación es mediante Vagrant**.

Se ha integrado un sistema de compilación automatizado y aislado que arranca una máquina virtual de Ubuntu, monta un disco ultrarrápido y realiza la generación de los tres binarios (`.exe` de Windows con metadatos incrustados, Linux ELF y macOS Mach-O). 

Para compilar el proyecto en todos los sistemas operativos simultáneamente de forma segura, solo tienes que ejecutar:

**En Linux / macOS:**
```bash
./compilar.sh --all
```

**En Windows:**
```powershell
.\compilar.ps1 -All
```
