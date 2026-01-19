# Nafuraito Kernel - Drivers Library
# Meta-makefile that builds all driver sub-libraries

SUBDIRS = video pic pci

.PHONY: all clean $(SUBDIRS)

all: $(SUBDIRS)

$(SUBDIRS):
	$(MAKE) -C $@

clean:
	@for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir clean; \
	done
