# Copyright (C) 2024 taylor.fish <contact@taylor.fish>
#
# This file is part of ncc.
#
# ncc is free software: you can redistribute it and/or modify it under
# the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# ncc is distributed in the hope that it will be useful, but WITHOUT
# ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
# or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General
# Public License for more details.
#
# You should have received a copy of the GNU Affero General Public
# License along with ncc. If not, see <https://www.gnu.org/licenses/>.

.PHONY: all
all: ../README.md package-readme.md

../README.md: README.m4 ../Cargo.toml
	m4 $< > $@

package-readme.md: README.m4 ../Cargo.toml
	m4 -DRUST $< > $@

.DELETE_ON_ERROR:
