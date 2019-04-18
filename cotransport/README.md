# cotransport

A seL4 application build and simulation tool.

This tool's job is to orchestrate the construction of seL4 applications.

It uses a [sel4 configuration format](../confignoble/README.md)
file sitting in a project's root dir to establish a canonical configuration
source and pipes that configuration, along with explicit output platform expectations
down through the application's build steps.