all-local:
	cd $(top_srcdir)/rust/emane-core && cargo build
	cp $(top_srcdir)/rust/target/debug/emane ./emane

clean-local:
	rm -f emane

install-exec-local:
	$(MKDIR_P) $(DESTDIR)$(bindir)
	$(INSTALL_PROGRAM) emane $(DESTDIR)$(bindir)/emane
