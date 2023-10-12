# Rumtopf

_Rumtopf_ is a generator for a static recipe website.
When provided with recipes in _Markdown_ format

# Building

`$ cargo build`

# Usage

## Recipe Sources

Recipes are written as _Markdown_ files.
The _Markdown_ parser is extended with additional syntax.

- `{{2 servings}}` instructs the generator to generate a form here that allows
  the visitor to adapt the number of servings.
- `{{12}}` instructs the website to adapt this number based on the number of
  servings specified in the form.

An example recipe is available at `recipes/pizza.md`.
Create your own recipes in the same way and place them inside a new directory.

## Generation

`$ ./rumtopf <SOURCE_DIR> <DESTINATION_DIR>`

- `SOURCE_DIR` is the directory with your _Markdown_ (`*.md`) recipes.
- `DESTINATION_DIR` must not exist yet and will be created by the generator.

See `--help` for more options.

`SOURCE_DIR` can additionally contain files not ending in `.md` which will be
copied verbatim to `DESTINATION_DIR`.
`DESTINATION_DIR` will now contain the generated HTML files along with some
static assets.

## Deployment

Simply copy the destination directory to your web server.
Enjoy!

## Use Case: Adding an Imprint

To add an imprint, first, add your `imprint.html` to your source directory with
your _Markdown_ recipes.
Add the option `--link Imprint=imprint.html` to the `rumtopf` command arguments.
This will generate a clickable link to the footer of every page.

## Use Case: Overriding Templates

The used HTML templates can be overridden.
These templates use the
[_Handlebars_](https://docs.rs/handlebars/latest/handlebars/index.html)
syntax.
The built-in ones can be found in `src/templates` and might be used as
reference.
To override templates, simply create a new directory and put the replacements as `.html` files in it.
The file name stems are taken as names for the templates.
Then provide the `--templates` option with the new directory as an additional
command-line argument to `rumtopf`.

# License

All sources of this project are licensed under the MIT license (see the
`LICENSE` file) unless otherwise noted.
