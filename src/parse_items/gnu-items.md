<!--
For the full copyright and license information, please view the LICENSE
file that was distributed with this source code.
-->

## General date syntax
https://www.gnu.org/software/coreutils/manual/html_node/General-date-syntax.html

A date string can have different flavours (items):
- calendar date
- time of day
- time zone
- combined date and time of day
- day of the week
- relative
- numbers
- empty string (beginning of the day)

Some properties:
- the order of items should not matter
- whitespace may be omitted when unambiguous
- ordinal numbers may be written out in some items
- comments between parentheses '(', ')'
- alphabetic case is ignored
- hyphens not followed by digit are ignored
- leading zeros on numbers are ignored
- leap seconds on supported systems
