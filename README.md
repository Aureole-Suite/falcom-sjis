# falcom-sjis

A Shift JIS decoder and encoder using Nihon Falcom's encoding tables, for
maximal compatibility with their games.

Several Falcom games, including the *Trails* series, encodes the ♥ character
using the codepoint for ㈱, and a few similar substitutions. This crate does
*not* handle these substitutions.
