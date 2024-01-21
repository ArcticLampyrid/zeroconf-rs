use crate::prelude::*;
use crate::{MdnsBrowser, MdnsService, ServiceType, TxtRecord};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

#[derive(Default, Debug)]
struct Context {
    is_discovered: bool,
    timed_out: bool,
    txt: Option<TxtRecord>,
}

#[test]
fn service_register_is_browsable() {
    super::setup();

    const TOTAL_TEST_TIME_S: u64 = 30;
    static SERVICE_NAME: &str = "service_register_is_browsable";

    let mut service = MdnsService::new(
        ServiceType::with_sub_types("http", "tcp", vec!["printer"]).unwrap(),
        8080,
    );

    let context: Rc<RefCell<Context>> = Rc::default();

    let mut txt = TxtRecord::new();
    txt.insert("foo", "bar").unwrap();

    service.set_name(SERVICE_NAME);
    service.set_context(Box::new(context.clone()));
    service.set_txt_record(txt.clone());

    service.set_registered_callback(Box::new(|_, context| {
        let mut browser =
            MdnsBrowser::new(ServiceType::with_sub_types("http", "tcp", vec!["printer"]).unwrap());

        browser.set_context(Box::new(context.clone()));

        browser.set_service_discovered_callback(Box::new(|service, context| {
            let service = service.unwrap();

            if service.name() == SERVICE_NAME {
                let c = context.as_ref().unwrap().borrow_mut();
                let mut context = c.downcast_ref::<RefCell<Context>>().unwrap().borrow_mut();

                context.txt = service.txt().clone();
                context.is_discovered = true;

                debug!("Service discovered");
            }
        }));

        let event_loop = browser.browse_services().unwrap();
        let browse_start = std::time::Instant::now();

        loop {
            event_loop.poll(Duration::from_secs(0)).unwrap();

            let mut c = context.as_ref().unwrap().borrow_mut();
            let context = c.downcast_mut::<Context>().unwrap();

            if context.is_discovered {
                break;
            }

            if browse_start.elapsed().as_secs() > TOTAL_TEST_TIME_S / 2 {
                context.timed_out = true;
                break;
            }
        }
    }));

    let event_loop = service.register().unwrap();
    let publish_start = std::time::Instant::now();

    loop {
        event_loop.poll(Duration::from_secs(0)).unwrap();

        let mut mtx = context.borrow_mut();

        if mtx.is_discovered {
            assert_eq!(txt, mtx.txt.take().unwrap());
            break;
        }

        if publish_start.elapsed().as_secs() > TOTAL_TEST_TIME_S {
            mtx.timed_out = true;
            break;
        }
    }

    assert!(!context.borrow().timed_out);
}
