# Entorno de Integración Continua (GitLab CI/CD)

LexiShield cuenta con un entorno preconfigurado basado en **Vagrant** y **Bento Debian 12** diseñado exclusivamente para ejecutar pipelines de compilación, pruebas y despliegue utilizando contenedores Docker.

Este entorno está separado del entorno de desarrollo principal para mantener el aislamiento y evitar consumo innecesario de recursos.

## 🚀 1. Arrancar la Máquina Virtual de CI/CD

El archivo de configuración Vagrant específico para GitLab se encuentra en `build/Vagrantfile.gitlab`.

Para iniciar esta máquina virtual, ejecuta desde la **raíz del proyecto** en tu sistema anfitrión:

```bash
VAGRANT_VAGRANTFILE=build/Vagrantfile.gitlab vagrant up
```

Este comando descargará la imagen (si no la tienes), asignará 4 vCPUs, 4GB de RAM y 2GB de memoria SWAP. Además, instalará automáticamente **Docker** y el agente **GitLab Runner**.

## 💻 2. Acceder a la Máquina Virtual

Una vez que la máquina haya arrancado, puedes entrar por SSH ejecutando:

```bash
VAGRANT_VAGRANTFILE=build/Vagrantfile.gitlab vagrant ssh
```

## 🔗 3. Registrar el Runner en tu Proyecto GitLab

Para que la VM comience a ejecutar los trabajos definidos en `.gitlab-ci.yml`, necesitas vincularla a tu proyecto en GitLab (ya sea en `gitlab.com` o en tu servidor privado).

1. Ve a tu proyecto en GitLab: **Settings $\rightarrow$ CI/CD $\rightarrow$ Runners**.
2. Selecciona **New project runner**.
3. Elige la plataforma **Linux** y haz clic en crear. Copia el **Token de registro** (ej. `glrt-abcdef123456`).
4. Dentro de la terminal SSH de tu VM, usa el script de ayuda preinstalado:

```bash
sudo register-runner glrt-abcdef123456
```

*(Nota: Si usas un servidor GitLab privado, añade la URL como segundo parámetro: `sudo register-runner <TOKEN> http://tu-gitlab.local`)*.

El Runner aparecerá activo (en verde) en la interfaz de GitLab y comenzará a procesar cualquier *push* o *tag* nuevo.

## 🦊 4. (Opcional) Lanzar un Servidor GitLab Local Completo

Si deseas trabajar de forma 100% offline o probar flujos sin depender de la nube, la VM incluye un script para desplegar instantáneamente **GitLab Community Edition** usando Docker.

Desde la terminal SSH de tu VM, ejecuta:

```bash
sudo start-gitlab-server
```

### Cómo acceder al servidor local:
1. Abre un navegador en tu máquina anfitriona y ve a: `http://localhost:8080`
2. El usuario por defecto es `root`.
3. Para obtener la contraseña inicial autogenerada, ejecuta en la VM:
   ```bash
   sudo docker exec -it gitlab grep 'Password:' /etc/gitlab/initial_root_password
   ```

## 🛑 5. Apagar o Suspender el Entorno

Cuando no estés compilando o ejecutando pipelines, puedes liberar los recursos de tu ordenador:

* **Suspender (pausa rápida guardando la memoria RAM):**
  ```bash
  VAGRANT_VAGRANTFILE=build/Vagrantfile.gitlab vagrant suspend
  ```

* **Apagar (cierre limpio del sistema):**
  ```bash
  VAGRANT_VAGRANTFILE=build/Vagrantfile.gitlab vagrant halt
  ```

* **Destruir (borrar la VM por completo perdiendo el Runner registrado):**
  ```bash
  VAGRANT_VAGRANTFILE=build/Vagrantfile.gitlab vagrant destroy
  ```
