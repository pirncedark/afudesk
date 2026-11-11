#include "my_application.h"
#include <flutter_linux/flutter_linux.h>
#include <libsecret/secret.h>
#include <cstring>
#include "flutter/generated_plugin_registrant.h"
#include "google_secure_storage.h"

struct _MyApplication {
  GtkApplication parent_instance;
  gchar** dart_entrypoint_arguments;
};
G_DEFINE_TYPE(MyApplication, my_application, GTK_TYPE_APPLICATION)

// Secret Service first; an absent service uses authenticated encrypted 0600 files.
static void secure_storage_call(FlMethodChannel*, FlMethodCall* call, gpointer) {
  const gchar* method = fl_method_call_get_name(call);
  g_autoptr(GError) error = nullptr;
  g_autoptr(FlMethodResponse) response = nullptr;
  if (std::strcmp(method, "read") == 0) {
    gchar* password = afudesk::ReadGoogleSession(&error);
    if (error == nullptr) {
      g_autoptr(FlValue) value = password == nullptr ? fl_value_new_null() : fl_value_new_string(password);
      response = FL_METHOD_RESPONSE(fl_method_success_response_new(value));
    }
    if (password != nullptr) secret_password_free(password);
  } else if (std::strcmp(method, "write") == 0) {
    FlValue* args = fl_method_call_get_args(call);
    if (args != nullptr && fl_value_get_type(args) == FL_VALUE_TYPE_STRING &&
        afudesk::WriteGoogleSession(fl_value_get_string(args), &error)) {
      response = FL_METHOD_RESPONSE(fl_method_success_response_new(nullptr));
    }
  } else if (std::strcmp(method, "clear") == 0) {
    if (afudesk::ClearGoogleSession(&error)) {
      response = FL_METHOD_RESPONSE(fl_method_success_response_new(nullptr));
    }
  } else {
    response = FL_METHOD_RESPONSE(fl_method_not_implemented_response_new());
  }
  if (response == nullptr) {
    response = FL_METHOD_RESPONSE(fl_method_error_response_new("secure_storage",
        "Oturum güvenle kaydedilemedi; uygulamayı yeniden açıp dene.", nullptr));
  }
  fl_method_call_respond(call, response, nullptr);
}

static void my_application_activate(GApplication* application) {
  MyApplication* self = MY_APPLICATION(application);
  GtkWindow* existing = gtk_application_get_active_window(GTK_APPLICATION(application));
  if (existing != nullptr) {
    gtk_window_present(existing);
    return;
  }
  GtkWindow* window = GTK_WINDOW(gtk_application_window_new(GTK_APPLICATION(application)));
  gtk_window_set_title(window, "AfuDesk");
  gtk_window_set_default_size(window, 1100, 760);
  g_autoptr(FlDartProject) project = fl_dart_project_new();
  fl_dart_project_set_dart_entrypoint_arguments(project, self->dart_entrypoint_arguments);
  FlView* view = fl_view_new(project);
  gtk_widget_show(GTK_WIDGET(view));
  gtk_container_add(GTK_CONTAINER(window), GTK_WIDGET(view));
  fl_register_plugins(FL_PLUGIN_REGISTRY(view));
  g_autoptr(FlStandardMethodCodec) codec = fl_standard_method_codec_new();
  g_autoptr(FlMethodChannel) secure_channel = fl_method_channel_new(
      fl_engine_get_binary_messenger(fl_view_get_engine(view)),
      "afudesk/secure_storage", FL_METHOD_CODEC(codec));
  fl_method_channel_set_method_call_handler(secure_channel, secure_storage_call, nullptr, nullptr);
  gtk_widget_grab_focus(GTK_WIDGET(view));
  gtk_widget_show(GTK_WIDGET(window));
}

static gboolean my_application_local_command_line(GApplication* application, gchar*** arguments, int* exit_status) {
  MyApplication* self = MY_APPLICATION(application);
  self->dart_entrypoint_arguments = g_strdupv(*arguments + 1);
  g_autoptr(GError) error = nullptr;
  if (!g_application_register(application, nullptr, &error)) {
    g_warning("Application registration failed: %s", error->message);
    *exit_status = 1;
    return TRUE;
  }
  g_application_activate(application);
  *exit_status = 0;
  return TRUE;
}

static void my_application_dispose(GObject* object) {
  MyApplication* self = MY_APPLICATION(object);
  g_clear_pointer(&self->dart_entrypoint_arguments, g_strfreev);
  G_OBJECT_CLASS(my_application_parent_class)->dispose(object);
}

static void my_application_class_init(MyApplicationClass* klass) {
  G_APPLICATION_CLASS(klass)->activate = my_application_activate;
  G_APPLICATION_CLASS(klass)->local_command_line = my_application_local_command_line;
  G_OBJECT_CLASS(klass)->dispose = my_application_dispose;
}
static void my_application_init(MyApplication*) {}
MyApplication* my_application_new() {
  return MY_APPLICATION(g_object_new(my_application_get_type(),
      "application-id", APPLICATION_ID,
      "flags", G_APPLICATION_DEFAULT_FLAGS, nullptr));
}
