# Courses

![Build badge](https://img.shields.io/github/actions/workflow/status/antonmeskildsen/courses/ci_courses.yml)
![Docs badge](https://img.shields.io/github/actions/workflow/status/antonmeskildsen/courses/docs.yml?label=documentation)
![Crates badge](https://img.shields.io/crates/v/courses)


Courses is a publishing system for interactive content written in `rust`. Its aimed primarily at code-related learning 
resources, including books, course materials, and blogs.

[Documentation page](https://antonmeskildsen.github.io/courses/)

*Important: This project is very early in its development process and may change significantly in scope or function before an 
initial stable version is released.* 


Features:
- Supports interactive source files (Jupyter Notebooks) as well as regular markup(Markdown).
- Define tasks and their solutions directly in source files. Courses is made specifically with learning resources in mind.
- Uses a simple template-like syntax (shortcodes) for applying built-in and custom components, including figures and admonitions.

## The purpose

Courses was created to solve a specific set of problems not covered by similar tools like Jupyterbook and Quarto. 
These tools are based on large ecosystems for performing the publishing process. In contrast, `courses` is 
light-weight, self-contained and is highly customizable and extendable.

## An integrated format for defining exercises. 
Most modern learning materials use exercises or tasks in some capacity. For technical materials involving code, 
keeping track of the solutions and any student provided code becomes increasingly complex and error-prone as a project 
grows in size. The naive approach of keeping a separate solution version is problematic because it needs to also 
copy the exercise definition. Any changes made to one version must therefore manually be copied to the other. 
Additionally, there's no easy way to test the published version (i.e. without the solution).

In courses, the exercise definitions and solutions (in content files or code files) can be defined in the same source 
file using a simple syntax to 
specify sections as e.g. the solution. Instead of having to manually keep track of changes, the tool uses the 
complete definition to test the solution and to build the output with a placeholder. Creating new exercises using 
this method is easy. You start from a solution and simply mark of the parts that should not be visible in the published output.

**Example code block:**
```python
#| << CODE
# print("this is the placeholder and will be shown in the published output")
#| >> SOLUTION <<
print("this is the solution which is removed in the published version")
#| >> END_CODE
```

*Note that the extra syntax is hidden in comments and therefore does not interfere with the underlying 
implementation language. The solution is the only part that is not commented and can therefore be run during 
development.*

### Shortcodes
This is a simple and generic syntax for extending the functionality of the basic Markdown format.

These are heavily inspired by Hugo and even moreso by Zola (which uses the same templating engine, Tera, as Courses). 

``` ```

## Speed is a feature
Courses is fast enough to make a real-time editing workflow possible. The built-in automatic rebuild and reload 
mechanism makes any change to the source files appear almost instantaneously in the browser. Even full builds 
typically take less than a second to complete, even for large projects.

You should be able to focus on writing content. A faster tool gets out of your way and gives you feedback as you 
make changes.


## Similar projects
Courses is far from the only tool for publishing learning resources and scientific material to the web. Depending on 
your use case, these tools may be more appropriate for you.

Choose `courses` if you
- Value automatic and almost instantaneous rebuilds for watching changes live.
- Write material that contains exercises that will benefit from automated testing and handling of solutions.
- Want a simple binary that just work and has no dependency on environment setup or other tools.
- Want an all-in-one tool for publishing your work. `courses` is not 
- Want a tool that is extensible
- Can accept using a very new, likely unstable and definitely unsupported project.

Choose Jupyterbook or Quarto if you
- Mainly work with the Python or R ecosystems and don't want to meddle with HTML and templates
- Just want to get your notebooks onto the web
- Need integration with Jupyterhub
- Need mature interactive components in your content


### Jupyterbook
This project focuses highly on tight integration with the Jupyter Notebook ecosystem and formats. It is especially 
great for allowing interaction with notebooks using existing technologies like JupyterHub. It is a quite 
mature project built on a very mature foundation including Sphinx for document handling and of course Jupyter for 
interactivity. However, the huge amount of features it supports makes it much more complex than something like 
`courses`. Because the entire project is built in Python, building is slow. Additionally, there's no 
convenient automatic reload function.

### Quarto


### Static-site generators (e.g. Hugo, Jekyll)
These tools are very generic and allow for customization of almost every aspect of the published webpage. Hugo is 
also very fast and widely used. However, these tools don't support Jupyter Notebooks or interactive content in 
general (at least not directly) and they also have no support for automated tests or exercise handling.

I heavily considered creating `courses` as an extension to one of these systems but ultimately chose not to because 
of how much complexity and clutter the extra pre- and post-processing would add. 