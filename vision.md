# The product vision for tldiagram.com and tld-cli

## tldiagram.com

A living visual documentation of your multi-repo architecture. Some diagrams in a user's workspace may be manually created, created by AI agents or created by other tools. We will focus on a seamless integration to work on a portion of their workspace using tld-cli.

**I envision a workflow like this**

Users will generate the high level overview of their architecture and link repositories in their organization to the diagram. They may chose to stop at Frontend <-> Backend <-> Database or go as deep as they want to. They will set associate Frontend with a repo link and designate it as root.

## tld-cli
tld-cli will be initialized in the root of a repository working locally. For example: the Frontend repository.
`tld analyze` command will scan the repo compile the source code symbols and call hierarchies. cli will use this information to create a diagram of the "Frontend" repository. 

example .tld.yaml file for the "E-commerce Platform" project with a "Frontend" repository:
```yaml
project_name: "E-commerce Platform"
repositories:
  frontend:
    url: github.com/example/frontend
    localDir: frontend
    root: bKLqGV48 # hashid of the root element in the diagram
    config:
      mode: auto # auto, upsert or manual
    exclude:
      - generated/**
```
## Config modes
### auto
tld-cli will automatically manage the diagram elements and their relationships based on the source code analysis. It will create new elements for new symbols and update existing ones as needed, deleting elements that no longer exist in the codebase.

### upsert
tld-cli will create new elements for new symbols and update existing ones, but it will not delete any elements. This allows users to maintain a history of their architecture changes over time.

### manual
tld-cli will not make any changes to the diagram elements or their relationships. Users will have full control over the diagram and must manually update it to reflect changes in the codebase.
