# Rumtopf

_Rumtopf_ is a generator for a static recipe website.
When provided with recipes in _Markdown_ format

# Building

`$ cargo build`

# Usage

## Recipe Sources

Recipes are creates as _Markdown_ files.
The _Markdown_ parser is extended with additional syntax.

- `{{2 servings}}` instructs the generator to generate a form here that allows
  the visitor to adapt the number of servings.
- `{{12}}` instructs the website to adapt this number based on the number of
  servings specified in the form.

An example recipe is available at `resources/pizza.md`.
Create your own recipes in the same way and place them inside a new directory.

## Generation

`$ ./rumtopf <SOURCE_DIR> <DESTINATION_DIR>`

- `SOURCE_DIR` is the directory with your _Markdown_ recipes.
- `DESTINATION_DIR` must not exist yet and will be created by the generator.

`DESTINATION_DIR` will now contain the generated HTML files.

## Deployment

You need to copy all non-_Markdown_ files of the `resources` directory into your
destination folder.
Finally, copy the destination directory to your webserver.
Enjoy!

# License

All sources of this project are licensed under the MIT license (see the
`LICENSE` file) unless otherwise noted.
