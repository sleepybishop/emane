#ifndef EMANE_XML_H
#define EMANE_XML_H

#include <string>

extern "C" {

struct EmaneXmlAttr {
    char* name;
    char* value;
    EmaneXmlAttr* next;
};

struct EmaneXmlNode {
    char* name;
    char* content;
    EmaneXmlNode* children;
    EmaneXmlNode* next;
    EmaneXmlAttr* attrs;
    bool type;
};

struct EmaneXmlDoc {
    EmaneXmlNode* root;
};

EmaneXmlDoc* emane_rs_xml_parse(const char* uri);
void emane_rs_xml_doc_free(EmaneXmlDoc* doc);
char* emane_rs_xml_get_prop(EmaneXmlNode* node, const char* name);
void emane_rs_xml_free_prop(char* prop);

}

typedef EmaneXmlNode xmlNode;
typedef EmaneXmlNode* xmlNodePtr;
typedef EmaneXmlDoc* xmlDocPtr;
typedef char xmlChar;

#define xmlDocGetRootElement(doc) (doc ? doc->root : nullptr)
#define xmlFreeDoc(doc) emane_rs_xml_doc_free(doc)
#define xmlFree(prop) emane_rs_xml_free_prop(prop)

inline bool xmlStrEqual(const xmlChar* a, const xmlChar* b) {
    if (!a || !b) return false;
    return std::string(reinterpret_cast<const char*>(a)) == reinterpret_cast<const char*>(b);
}

inline char* xmlGetProp(xmlNodePtr node, const xmlChar* name) {
    return emane_rs_xml_get_prop(node, reinterpret_cast<const char*>(name));
}

#define XML_ELEMENT_NODE true

#endif
