## Run `./bootstrap` to generate the "Makefile.in" files in this directory and
## the "$SUBDIRS" subdirectories.

SUBDIRS = include src . tests

EXTRA_DIST = Changes ReadMe.md License CMakeLists.txt doc/doxygen.cfg

LIBYAML_TEST_SUITE_RUN_REPO_DEFAULT := https://github.com/yaml/libyaml
LIBYAML_TEST_SUITE_RUN_REPO ?= $(LIBYAML_TEST_SUITE_RUN_REPO_DEFAULT)
LIBYAML_TEST_SUITE_RUN_BRANCH ?= run-test-suite
# Imported source snapshots like this ported checkout do not share libyaml's
# upstream git history, so the yaml-test-suite runner cannot infer a matching
# pin from merge ancestry here. Default to the last known-good pair instead.
LIBYAML_TEST_SUITE_CODE_COMMIT_DEFAULT := 67365dd2af265f17ba6ee0832c1411e4fd80ac0b
LIBYAML_TEST_SUITE_DATA_COMMIT_DEFAULT := data-2020-02-11
LIBYAML_TEST_SUITE_CODE_COMMIT ?= $(LIBYAML_TEST_SUITE_CODE_COMMIT_DEFAULT)
LIBYAML_TEST_SUITE_DATA_COMMIT ?= $(LIBYAML_TEST_SUITE_DATA_COMMIT_DEFAULT)

pkgconfigdir = $(libdir)/pkgconfig
pkgconfig_DATA = yaml-0.1.pc

maintainer-clean-local:
	rm -f aclocal.m4 config.h.in configure config/*
	-find ${builddir} -name Makefile.in -exec rm -f '{}' ';'

distclean-local:
	rm -fr tests/run-test-suite packaging
	-git worktree prune

.PHONY: bootstrap
bootstrap: maintainer-clean
	./bootstrap
	./configure
	make

test: all
	make -C tests check-TESTS

test-suite: tests/run-test-suite all
	LIBYAML_TEST_SUITE_CODE_COMMIT=$(LIBYAML_TEST_SUITE_CODE_COMMIT) \
	LIBYAML_TEST_SUITE_DATA_COMMIT=$(LIBYAML_TEST_SUITE_DATA_COMMIT) \
	make -C $< test

test-all: test test-suite

tests/run-test-suite:
ifeq ($(LIBYAML_TEST_SUITE_RUN_REPO),$(LIBYAML_TEST_SUITE_RUN_REPO_DEFAULT))
	  if git show-ref --verify --quiet refs/heads/$(LIBYAML_TEST_SUITE_RUN_BRANCH) || \
	     git show-ref --verify --quiet refs/remotes/origin/$(LIBYAML_TEST_SUITE_RUN_BRANCH); then \
	    git branch --track $(LIBYAML_TEST_SUITE_RUN_BRANCH) origin/$(LIBYAML_TEST_SUITE_RUN_BRANCH) >/dev/null 2>&1 || true; \
	    git worktree prune; \
	    git worktree add $@ $(LIBYAML_TEST_SUITE_RUN_BRANCH); \
	  else \
	    git clone --branch $(LIBYAML_TEST_SUITE_RUN_BRANCH) $(LIBYAML_TEST_SUITE_RUN_REPO) $@; \
	  fi
    else
	  git clone --branch $(LIBYAML_TEST_SUITE_RUN_BRANCH) $(LIBYAML_TEST_SUITE_RUN_REPO) $@
    endif

packaging:
	-git branch --track $@ origin/$@
	git worktree add --force $@ $@

docker-dist: packaging
	make -C $</docker libyaml-dist

docker-test-pyyaml: packaging
	make -C $</docker test-pyyaml
