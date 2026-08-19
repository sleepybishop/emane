lib_LTLIBRARIES = librfpipemaclayer.la

librfpipemaclayer_la_CPPFLAGS = \
 -I@top_srcdir@/include \
 $(AM_CPPFLAGS) \
 $(libuuid_CFLAGS)

librfpipemaclayer_la_LIBADD = \
 $(libuuid_LIBS)

librfpipemaclayer_la_LDFLAGS = \
 $(AM_LDFLAGS) \
 -avoid-version

librfpipemaclayer_la_SOURCES = maclayer.cc

edit = sed -e 's|@datadir[@]|$(pkgdatadir)|g'  

rfpipenem.xml: rfpipenem.xml.in
	$(edit) $< > $@

rfpipemac.xml: rfpipemac.xml.in
	$(edit) $< > $@

rfpipepcr.xml: rfpipepcr.xml.in
	$(edit) $< > $@

uninstall-local:
	rm -f $(DESTDIR)$(datadir)/$(PACKAGE)/xml/models/mac/rfpipe/rfpipenem.xml
	rm -f $(DESTDIR)$(datadir)/$(PACKAGE)/xml/models/mac/rfpipe/rfpipemac.xml
	rm -f $(DESTDIR)$(datadir)/$(PACKAGE)/xml/models/mac/rfpipe/rfpipepcr.xml
