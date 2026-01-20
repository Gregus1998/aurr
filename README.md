# AURR - A Yggdrasil soil project

## Tips and Triks: .devcontainer/devcontainer.json

Dont know if we need: .devcontainer/devcontainer.json 
    -> but it makes it possible to run the code on any system with docker. Sounds nice. 
    -> Need to create a a run script based on the: .devcontainer/devcontainer.

Source: https://bkedwards.github.io/comp423-course-notes/tutorials/rust-setup/
name: A descriptive name for your dev container.

image: The Docker image to use, in this case, the latest version of a Rust environment. Microsoft maintains a collection of base images for many programming language environments, but you can also create your own!

customizations: Adds useful configurations to VS Code, like installing the Rust extension. When you search for VSCode extensions on the marketplace, you will find the string identifier of each extension in its sidebar. Adding extensions here ensures other developers on your project have them installed in their dev containers automatically.

postCreateCommand: Commands to be executed after the container is created. In our case, there is nothing to be run after creation.