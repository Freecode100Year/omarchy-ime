#include <fcitx/addonfactory.h>
#include <fcitx/addonmanager.h>
#include <fcitx/candidatelist.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextproperty.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/inputpanel.h>
#include <fcitx/instance.h>
#include <fcitx/text.h>
#include <fcitx-utils/keysym.h>
#include "omarchy_ime.h"
#include <memory>
#include <cctype>
#include <string>
class OmarchyState final : public fcitx::InputContextProperty {
public:
 explicit OmarchyState(fcitx::InputContext *ic):ic_(ic),engine_(omarchy_ime_engine_new()){}
 ~OmarchyState() override { omarchy_ime_engine_free(engine_); }
 bool handle(fcitx::KeyEvent &); void update(); void choose(size_t);
private: fcitx::InputContext *ic_; EngineHandle *engine_;
 friend class OmarchyEngine;
};
class OmarchyCandidate final : public fcitx::CandidateWord {
public: OmarchyCandidate(OmarchyState *s,std::string t,size_t i):state_(s),index_(i){setText(fcitx::Text(std::move(t)));}
 void select(fcitx::InputContext *) const override { state_->choose(index_); }
private: OmarchyState *state_; size_t index_;
};
class OmarchyEngine final : public fcitx::InputMethodEngineV2 {
public:
 explicit OmarchyEngine(fcitx::Instance *i):factory_([](fcitx::InputContext &ic){return new OmarchyState(&ic);}) { i->inputContextManager().registerProperty("omarchyImeState",&factory_); }
 void keyEvent(const fcitx::InputMethodEntry &,fcitx::KeyEvent &e) override { if(!e.isRelease()) eventState(e)->handle(e); }
 void reset(const fcitx::InputMethodEntry &,fcitx::InputContextEvent &e) override { auto*s=e.inputContext()->propertyFor(&factory_); omarchy_ime_engine_key(s->engine_,0x102,0,false); s->update(); }
private:
 OmarchyState *eventState(fcitx::KeyEvent &e){return e.inputContext()->propertyFor(&factory_);}
 fcitx::FactoryFor<OmarchyState> factory_;
};
bool OmarchyState::handle(fcitx::KeyEvent &event) {
 auto key=event.key();
 const bool secure=ic_->capabilityFlags().testAny(fcitx::CapabilityFlag::PasswordOrSensitive);
 if(secure){omarchy_ime_engine_key(engine_,0,0,true);update();return false;}
 auto states=key.states(); if(states.test(fcitx::KeyState::Ctrl)||states.test(fcitx::KeyState::Alt)||states.test(fcitx::KeyState::Super)) return false;
 uint32_t value=0;
 if(key.check(FcitxKey_BackSpace)) value=0x100; else if(key.check(FcitxKey_Return)||key.check(FcitxKey_KP_Enter)) value=0x101;
 else if(key.check(FcitxKey_Escape)) value=0x102; else if(key.check(FcitxKey_space)||key.check(FcitxKey_KP_Space)) value=0x103;
 else if(key.isDigit()){
  auto d=key.digit();size_t i=d==0?9:d-1;
  if(i<omarchy_ime_engine_candidate_count(engine_)){choose(i);event.filterAndAccept();return true;}
  const char *pending=omarchy_ime_engine_preedit(engine_);
  if(pending&&*pending){int flush=omarchy_ime_engine_key(engine_,0x103,0,false);if(flush==2){const char*t=omarchy_ime_engine_commit(engine_);if(t)ic_->commitString(t);}update();}
  return false;
}
 else {
  auto c=fcitx::Key::keySymToUnicode(key.sym());
  const char *pending=omarchy_ime_engine_preedit(engine_);
  const bool printable_non_ascii=c>0x7f;
  if(!c)return false;
  // Commit Chinese composition before allowing a delimiter to reach the client.
  if((printable_non_ascii || !std::isalnum(static_cast<unsigned char>(c))) && pending && *pending){
   int flush=omarchy_ime_engine_key(engine_,0x103,0,false);
   if(flush==2){const char*t=omarchy_ime_engine_commit(engine_);if(t)ic_->commitString(t);}
   update();return false;
  }
  if(printable_non_ascii)return false;
  value=c;
 }
 int result=omarchy_ime_engine_key(engine_,value,0,false);if(result<0){update();return false;}event.filterAndAccept();
 if(result==2){const char*t=omarchy_ime_engine_commit(engine_);if(t)ic_->commitString(t);} update();return true;
}
void OmarchyState::choose(size_t index){
 if(ic_->capabilityFlags().testAny(fcitx::CapabilityFlag::PasswordOrSensitive)){omarchy_ime_engine_key(engine_,0,0,true);update();return;}
 int r=omarchy_ime_engine_select(engine_,index);if(r==2){const char*t=omarchy_ime_engine_commit(engine_);if(t)ic_->commitString(t);}update();}
void OmarchyState::update(){auto&p=ic_->inputPanel();p.reset();auto list=std::make_unique<fcitx::CommonCandidateList>();list->setPageSize(9);list->setLabels({"1.","2.","3.","4.","5.","6.","7.","8.","9."});auto n=omarchy_ime_engine_candidate_count(engine_);
 for(size_t i=0;i<n;i++){const char*t=omarchy_ime_engine_candidate(engine_,i);if(t)list->append<OmarchyCandidate>(this,std::string(t),i);}if(n)p.setCandidateList(std::move(list));
 const char*t=omarchy_ime_engine_preedit(engine_);if(t&&*t){fcitx::Text text(t);if(ic_->capabilityFlags().test(fcitx::CapabilityFlag::Preedit))p.setClientPreedit(text);else p.setPreedit(text);}
 ic_->updateUserInterface(fcitx::UserInterfaceComponent::InputPanel);ic_->updatePreedit();}
class OmarchyFactory final:public fcitx::AddonFactory{fcitx::AddonInstance*create(fcitx::AddonManager*m)override{return new OmarchyEngine(m->instance());}};
FCITX_ADDON_FACTORY(OmarchyFactory);
