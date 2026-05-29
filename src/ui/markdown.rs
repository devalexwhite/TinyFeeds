use std::rc::Rc;
use std::cell::RefCell;
use gtk4::{self as gtk, glib};
use gtk4::prelude::*;
use crate::models::{AppState, AppMessage, ImageWidgetRef};
use crate::image_loader::download_image;

pub fn clear_container(container: &gtk::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&apos;")
}

pub fn render_markdown(
    markdown: &str, 
    container: &gtk::Box, 
    app_state: &Rc<RefCell<AppState>>,
    tx: &tokio::sync::mpsc::UnboundedSender<AppMessage>
) {
    let parser = pulldown_cmark::Parser::new(markdown);
    
    let mut current_markup = String::new();
    let mut is_ordered = false;
    let mut list_index = 0;
    
    let mut in_blockquote = false;
    let mut blockquote_markup = String::new();
    
    let mut in_code_block = false;
    let mut code_block_text = String::new();
    
    for event in parser {
        match event {
            pulldown_cmark::Event::Start(tag) => {
                match tag {
                    pulldown_cmark::Tag::Heading { .. } => {
                        current_markup.clear();
                    }
                    pulldown_cmark::Tag::Paragraph => {
                        if !in_blockquote {
                            current_markup.clear();
                        }
                    }
                    pulldown_cmark::Tag::BlockQuote(_) => {
                        in_blockquote = true;
                        blockquote_markup.clear();
                    }
                    pulldown_cmark::Tag::CodeBlock(..) => {
                        in_code_block = true;
                        code_block_text.clear();
                    }
                    pulldown_cmark::Tag::List(ordered_start) => {
                        is_ordered = ordered_start.is_some();
                        list_index = ordered_start.unwrap_or(1) as usize;
                    }
                    pulldown_cmark::Tag::Item => {
                        current_markup.clear();
                        if is_ordered {
                            current_markup.push_str(&format!("{}. ", list_index));
                            list_index += 1;
                        } else {
                            current_markup.push_str("• ");
                        }
                    }
                    pulldown_cmark::Tag::Emphasis => {
                        current_markup.push_str("<i>");
                    }
                    pulldown_cmark::Tag::Strong => {
                        current_markup.push_str("<b>");
                    }
                    pulldown_cmark::Tag::Strikethrough => {
                        current_markup.push_str("<s>");
                    }
                    pulldown_cmark::Tag::Link { dest_url, .. } => {
                        current_markup.push_str(&format!("<a href=\"{}\">", escape_html(&dest_url)));
                    }
                    pulldown_cmark::Tag::Image { dest_url, .. } => {
                        let stack = gtk::Stack::new();
                        stack.set_transition_type(gtk::StackTransitionType::Crossfade);
                        
                        let spinner = gtk::Spinner::new();
                        spinner.start();
                        spinner.set_halign(gtk::Align::Center);
                        spinner.set_valign(gtk::Align::Center);
                        
                        let picture = gtk::Picture::new();
                        picture.set_keep_aspect_ratio(true);
                        picture.set_halign(gtk::Align::Center);
                        picture.set_margin_top(12);
                        picture.set_margin_bottom(12);
                        picture.set_height_request(200);
                        
                        let error_label = gtk::Label::new(Some("Failed to load image"));
                        error_label.add_css_class("dim-label");
                        
                        stack.add_child(&spinner);
                        stack.add_child(&picture);
                        stack.add_child(&error_label);
                        
                        stack.set_visible_child(&spinner);
                        
                        container.append(&stack);
                        
                        let mut state = app_state.borrow_mut();
                        state.image_widgets.entry(dest_url.to_string()).or_default().push(ImageWidgetRef {
                            stack: stack.clone(),
                            picture: picture.clone(),
                            error_label: error_label.clone(),
                        });
                        
                        if state.images_in_progress.insert(dest_url.to_string()) {
                            let url = dest_url.to_string();
                            let base_url = state.stories.first().map(|s| s.url.clone());
                            let tx_c = tx.clone();
                            
                            tokio::spawn(async move {
                                match download_image(url.clone(), base_url).await {
                                    Ok(img) => {
                                        tx_c.send(AppMessage::ImageDownloaded { url, bytes: img.bytes }).ok();
                                    }
                                    Err(err) => {
                                        tx_c.send(AppMessage::ImageDownloadFailed { url, error: err }).ok();
                                    }
                                }
                            });
                        }
                    }
                    _ => {}
                }
            }
            pulldown_cmark::Event::End(tag) => {
                match tag {
                    pulldown_cmark::TagEnd::Heading(level) => {
                        let label = gtk::Label::new(None);
                        let markup = match level {
                            pulldown_cmark::HeadingLevel::H1 => format!("<span size=\"xx-large\" weight=\"bold\">{}</span>", current_markup),
                            pulldown_cmark::HeadingLevel::H2 => format!("<span size=\"x-large\" weight=\"bold\">{}</span>", current_markup),
                            pulldown_cmark::HeadingLevel::H3 => format!("<span size=\"large\" weight=\"bold\">{}</span>", current_markup),
                            _ => format!("<span size=\"medium\" weight=\"bold\">{}</span>", current_markup),
                        };
                        label.set_markup(&markup);
                        label.set_wrap(true);
                        label.set_halign(gtk::Align::Start);
                        label.set_selectable(true);
                        label.set_margin_top(16);
                        label.set_margin_bottom(8);
                        container.append(&label);
                    }
                    pulldown_cmark::TagEnd::Paragraph => {
                        if !in_blockquote {
                            let label = gtk::Label::new(None);
                            label.set_markup(&current_markup);
                            label.set_wrap(true);
                            label.set_halign(gtk::Align::Start);
                            label.set_selectable(true);
                            label.set_margin_bottom(12);
                            label.connect_activate_link(|_, uri| {
                                webbrowser::open(uri).ok();
                                glib::Propagation::Stop
                            });
                            container.append(&label);
                        }
                    }
                    pulldown_cmark::TagEnd::BlockQuote(_) => {
                        in_blockquote = false;
                        let label = gtk::Label::new(None);
                        label.set_markup(&blockquote_markup);
                        label.set_wrap(true);
                        label.set_halign(gtk::Align::Start);
                        label.set_selectable(true);
                        label.add_css_class("blockquote");
                        label.set_margin_bottom(12);
                        label.connect_activate_link(|_, uri| {
                            webbrowser::open(uri).ok();
                            glib::Propagation::Stop
                        });
                        container.append(&label);
                    }
                    pulldown_cmark::TagEnd::CodeBlock => {
                        in_code_block = false;
                        let label = gtk::Label::new(Some(&code_block_text));
                        label.set_wrap(true);
                        label.set_halign(gtk::Align::Start);
                        label.set_selectable(true);
                        label.add_css_class("code-block");
                        label.set_margin_bottom(12);
                        container.append(&label);
                    }
                    pulldown_cmark::TagEnd::List(..) => {}
                    pulldown_cmark::TagEnd::Item => {
                        let label = gtk::Label::new(None);
                        label.set_markup(&current_markup);
                        label.set_wrap(true);
                        label.set_halign(gtk::Align::Start);
                        label.set_selectable(true);
                        label.set_margin_start(20);
                        label.set_margin_bottom(6);
                        label.connect_activate_link(|_, uri| {
                            webbrowser::open(uri).ok();
                            glib::Propagation::Stop
                        });
                        container.append(&label);
                    }
                    pulldown_cmark::TagEnd::Emphasis => {
                        current_markup.push_str("</i>");
                    }
                    pulldown_cmark::TagEnd::Strong => {
                        current_markup.push_str("</b>");
                    }
                    pulldown_cmark::TagEnd::Strikethrough => {
                        current_markup.push_str("</s>");
                    }
                    pulldown_cmark::TagEnd::Link => {
                        current_markup.push_str("</a>");
                    }
                    _ => {}
                }
            }
            pulldown_cmark::Event::Text(text) => {
                let escaped = escape_html(&text);
                if in_code_block {
                    code_block_text.push_str(&text);
                } else if in_blockquote {
                    blockquote_markup.push_str(&escaped);
                } else {
                    current_markup.push_str(&escaped);
                }
            }
            pulldown_cmark::Event::Code(text) => {
                let escaped = escape_html(&text);
                let formatted = format!("<tt>{}</tt>", escaped);
                if in_blockquote {
                    blockquote_markup.push_str(&formatted);
                } else {
                    current_markup.push_str(&formatted);
                }
            }
            pulldown_cmark::Event::SoftBreak | pulldown_cmark::Event::HardBreak => {
                if in_blockquote {
                    blockquote_markup.push('\n');
                } else {
                    current_markup.push('\n');
                }
            }
            pulldown_cmark::Event::Rule => {
                let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
                separator.set_margin_top(12);
                separator.set_margin_bottom(12);
                container.append(&separator);
            }
            _ => {}
        }
    }
}
