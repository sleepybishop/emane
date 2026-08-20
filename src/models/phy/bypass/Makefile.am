EXTRA_DIST = bypassphy.xml.in
BUILT_SOURCES = bypassphy.xml

edit = sed -e 's|@datadir[@]|$(pkgdatadir)|g'

all-local:
	cd $(top_srcdir)/rust/emane-core && cargo build -p bypass-phy

bypassphy.xml: bypassphy.xml.in
	$(edit) $< > $@

clean-local:
	rm -f $(BUILT_SOURCES)

install-exec-hook:	
	$(mkinstalldirs) $(DESTDIR)$(datadir)/$(PACKAGE)/xml/models/phy/bypass
	cp -f bypassphy.xml $(SCHEMAS) $(DESTDIR)$(datadir)/$(PACKAGE)/xml/models/phy/bypass
	$(mkinstalldirs) $(DESTDIR)$(libdir)
	cp -f $(top_srcdir)/rust/target/debug/libbypassphylayer.so $(DESTDIR)$(libdir)/libbypassphylayer.so

uninstall-local:
	rm -f $(DESTDIR)$(datadir)/$(PACKAGE)/xml/models/phy/bypass/bypassphy.xml
	rm -f $(DESTDIR)$(libdir)/libbypassphylayer.so
